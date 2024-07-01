use std::str::FromStr;

use cosmwasm_std::{
    Addr, Binary, Coin, Decimal, Deps, DepsMut, Empty, entry_point, Env, MessageInfo,
    Reply, Response, SubMsg, SubMsgResult, to_json_binary, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use pryzm_std::types::pryzm::{
    amm::v1::{MsgBatchSwapResponse, SwapStep},
    icstaking::v1::MsgStakeResponse,
};

use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, ExecuteMsg, get_ask_denom_for_routes, InstantiateMsg,
        MigrateMsg, QueryMsg, Route, SimulateSmartSwapExactAssetInResponse,
        SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation,
    },
};

use crate::{
    error::{ContractError, ContractResult},
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_SWAP_OPERATIONS},
    swap::{parse_coin, SwapExecutionStep},
};
use crate::state::IN_PROGRESS_SWAP_SENDER;

const BATCH_SWAP_REPLY_ID: u64 = 1;
const STAKE_REPLY_ID: u64 = 2;

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    unimplemented!()
}

///////////////////
/// INSTANTIATE ///
///////////////////

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Swap { operations } => execute_swap(deps, env, info, operations),
        ExecuteMsg::TransferFundsBack {
            swapper,
            return_denom,
        } => Ok(execute_transfer_funds_back(
            deps,
            env,
            info,
            swapper,
            return_denom,
        )?),
        _ => {
            unimplemented!()
        }
    }
}

// Executes a swap with the given swap operations and then transfers the funds back to the caller
fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get coin in from the message info, error if there is not exactly one coin sent
    let coin_in = one_coin(&info)?;

    let mut execution_steps: Vec<SwapExecutionStep> = Vec::new();
    let mut amm_swap_steps: Vec<SwapStep> = Vec::new();

    let mut swap_operations_iter = operations.iter();
    while let Some(swap_op) = swap_operations_iter.next() {
        if swap_op.pool.starts_with("icstaking:") {
            if amm_swap_steps.len() != 0 {
                execution_steps.push(SwapExecutionStep::Swap {
                    swap_steps: amm_swap_steps.clone(),
                });
                amm_swap_steps = Vec::new();
            }
            let split = swap_op.pool.split(":"); // TODO validate
            execution_steps.push(SwapExecutionStep::Stake {
                host_chain_id: split[1],
                transfer_channel: split[2],
            });
        } else if swap_op.pool.starts_with("amm") {
            let pool = swap_op.pool.replace("amm:", "").parse()?;
            amm_swap_steps.push(SwapStep {
                pool_id: pool,
                token_in: swap_op.denom_in.clone(),
                token_out: swap_op.denom_out.clone(),
                amount: None,
            });
        } else {
            return Err(ContractError::InvalidPool);
        }
    }

    return execute_steps(deps, env, info.sender, coin_in, execution_steps);
}

fn execute_steps(
    deps: DepsMut,
    env: Env,
    swapper: Addr,
    coin_in: Coin,
    execution_steps: Vec<SwapExecutionStep>,
) -> ContractResult<Response> {
    // return error if execution_steps is empty
    if execution_steps.is_empty() {
        return Err(ContractError::SwapOperationsEmpty);
    }

    let step = execution_steps.first().unwrap();
    let msg = step.to_cosmos_msg(env.contract.address.to_string(), coin_in)?;

    if execution_steps.len() == 1 {
        // Create the transfer funds back message
        let transfer_funds_back_msg = WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                swapper,
                return_denom,
            })?,
            funds: vec![],
        };

        return Ok(Response::new()
            .add_message(msg.clone())
            .add_message(transfer_funds_back_msg)
            .add_attribute("action", "dispatch_swap_and_transfer_back"));
    }

    let sub_msg = match step {
        SwapExecutionStep::Swap => SubMsg::reply_on_success(msg.clone(), BATCH_SWAP_REPLY_ID),
        SwapExecutionStep::Stake => SubMsg::reply_on_success(msg.clone(), STAKE_REPLY_ID),
    };

    IN_PROGRESS_SWAP_OPERATIONS.save(deps.storage, &execution_steps)?;

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "dispatch_swap_step"))
}

/////////////
/// REPLY ///
/////////////

// Handles the reply from the swap step execution messages
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> ContractResult<Response> {
    // Get the sub message response from the reply and error if it does not exist
    // This should never happen since sub msg was set to reply on success only,
    // but added in case the wasm module doesn't behave as expected.
    let SubMsgResult::Ok(sub_msg_response) = reply.result else {
        unreachable!()
    };

    let coin_in: Coin;
    match reply.id {
        BATCH_SWAP_REPLY_ID => {
            // Parse the response from the sub message
            let resp: MsgBatchSwapResponse = MsgBatchSwapResponse::decode(
                sub_msg_response
                    .data
                    .ok_or(ContractError::MissingResponseData)?
                    .as_slice(),
            )?;
            coin_in = parse_coin(*resp.amounts_out.first()?.clone())?
        }
        STAKE_REPLY_ID => {
            // Parse the response from the sub message
            let resp: MsgStakeResponse = MsgStakeResponse::decode(
                sub_msg_response
                    .data
                    .ok_or(ContractError::MissingResponseData)?
                    .as_slice(),
            )?;
            coin_in = parse_coin(resp.c_amount?)?
        }
        _ => {
            // Error if the reply id is not the same as the one used in the sub message dispatched
            // This should never happen since we are using a constant reply id, but added in case
            // the wasm module doesn't behave as expected.
            unreachable!()
        }
    }

    let in_progress_exec_steps = IN_PROGRESS_SWAP_OPERATIONS.load(deps.storage)?;
    IN_PROGRESS_SWAP_OPERATIONS.remove(deps.storage);

    let swapper = IN_PROGRESS_SWAP_SENDER.load(deps.storage)?;
    IN_PROGRESS_SWAP_SENDER.remove(deps.storage);

    let mut new_steps = in_progress_exec_steps.clone();
    new_steps.remove(0); // TODO use a reversed stack for better performance
    execute_steps(deps, env, swapper, coin_in, new_steps)?;

    Ok(Response::new().add_attribute("action", "sub_msg_reply_success"))
}

// /////////////
// /// QUERY ///
// /////////////
//
// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
//     match msg {
//         QueryMsg::SimulateSwapExactAssetIn {
//             asset_in,
//             swap_operations,
//         } => to_json_binary(&query_simulate_swap_exact_asset_in(
//             deps,
//             asset_in,
//             swap_operations,
//         )?),
//         QueryMsg::SimulateSwapExactAssetOut {
//             asset_out,
//             swap_operations,
//         } => to_json_binary(&query_simulate_swap_exact_asset_out(
//             deps,
//             asset_out,
//             swap_operations,
//         )?),
//         QueryMsg::SimulateSwapExactAssetInWithMetadata {
//             asset_in,
//             swap_operations,
//             include_spot_price,
//         } => to_json_binary(&query_simulate_swap_exact_asset_in_with_metadata(
//             deps,
//             asset_in,
//             swap_operations,
//             include_spot_price,
//         )?),
//         QueryMsg::SimulateSwapExactAssetOutWithMetadata {
//             asset_out,
//             swap_operations,
//             include_spot_price,
//         } => to_json_binary(&query_simulate_swap_exact_asset_out_with_metadata(
//             deps,
//             asset_out,
//             swap_operations,
//             include_spot_price,
//         )?),
//         QueryMsg::SimulateSmartSwapExactAssetIn { routes, .. } => {
//             let ask_denom = get_ask_denom_for_routes(&routes)?;
//
//             to_json_binary(&query_simulate_smart_swap_exact_asset_in(
//                 deps, ask_denom, routes,
//             )?)
//         }
//         QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
//             routes,
//             asset_in,
//             include_spot_price,
//         } => {
//             let ask_denom = get_ask_denom_for_routes(&routes)?;
//
//             to_json_binary(&query_simulate_smart_swap_exact_asset_in_with_metadata(
//                 deps,
//                 asset_in,
//                 ask_denom,
//                 routes,
//                 include_spot_price,
//             )?)
//         }
//     }
//     .map_err(From::from)
// }
//
// // Queries the osmosis poolmanager module to simulate a swap exact amount in
// fn query_simulate_swap_exact_asset_in(
//     deps: Deps,
//     asset_in: Asset,
//     swap_operations: Vec<SwapOperation>,
// ) -> ContractResult<Asset> {
//     // Error if swap operations is empty
//     let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
//         return Err(ContractError::SwapOperationsEmpty);
//     };
//
//     // Get coin in from asset in, error if asset in is not a
//     // native coin because Osmosis does not support CW20 tokens.
//     let coin_in = match asset_in {
//         Asset::Native(coin) => coin,
//         _ => return Err(ContractError::AssetNotNative),
//     };
//
//     // Ensure coin_in's denom is the same as the first swap operation's denom in
//     if coin_in.denom != first_op.denom_in {
//         return Err(ContractError::CoinInDenomMismatch);
//     }
//
//     // Get denom out from last swap operation  to be used as the return coin's denom
//     let denom_out = last_op.denom_out.clone();
//
//     // Convert the swap operations to osmosis swap amount in routes
//     // Return an error if there was an error converting the swap
//     // operations to osmosis swap amount in routes.
//     let osmosis_swap_amount_in_routes: Vec<SwapAmountInRoute> =
//         convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;
//
//     // Query the osmosis poolmanager module to simulate the swap exact amount in
//     let res: EstimateSwapExactAmountInResponse = PoolmanagerQuerier::new(&deps.querier)
//         .estimate_swap_exact_amount_in(
//             osmosis_swap_amount_in_routes.first().unwrap().pool_id,
//             coin_in.to_string(),
//             osmosis_swap_amount_in_routes,
//         )?;
//
//     // Return the asset out
//     Ok(Coin {
//         denom: denom_out,
//         amount: Uint128::from_str(&res.token_out_amount)?,
//     }
//     .into())
// }
//
// // Queries the osmosis poolmanager module to simulate a swap exact amount out
// fn query_simulate_swap_exact_asset_out(
//     deps: Deps,
//     asset_out: Asset,
//     swap_operations: Vec<SwapOperation>,
// ) -> ContractResult<Asset> {
//     // Error if swap operations is empty
//     let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
//         return Err(ContractError::SwapOperationsEmpty);
//     };
//
//     // Get coin out from asset out, error if asset out is not a
//     // native coin because Osmosis does not support CW20 tokens.
//     let coin_out = match asset_out {
//         Asset::Native(coin) => coin,
//         _ => return Err(ContractError::AssetNotNative),
//     };
//
//     // Ensure coin_out's denom is the same as the last swap operation's denom out
//     if coin_out.denom != last_op.denom_out {
//         return Err(ContractError::CoinOutDenomMismatch);
//     }
//
//     // Get denom in from first swap operation to be used as the return coin's denom
//     let denom_in = first_op.denom_in.clone();
//
//     // Convert the swap operations to osmosis swap amount out routes
//     // Return an error if there was an error converting the swap
//     // operations to osmosis swap amount out routes.
//     let osmosis_swap_amount_out_routes: Vec<SwapAmountOutRoute> =
//         convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;
//
//     // Query the osmosis poolmanager module to simulate the swap exact amount out
//     let res: EstimateSwapExactAmountOutResponse = PoolmanagerQuerier::new(&deps.querier)
//         .estimate_swap_exact_amount_out(
//             osmosis_swap_amount_out_routes.first().unwrap().pool_id,
//             osmosis_swap_amount_out_routes,
//             coin_out.to_string(),
//         )?;
//
//     // Return the asset in needed
//     Ok(Coin {
//         denom: denom_in,
//         amount: Uint128::from_str(&res.token_in_amount)?,
//     }
//     .into())
// }
//
// fn query_simulate_smart_swap_exact_asset_in(
//     deps: Deps,
//     ask_denom: String,
//     routes: Vec<Route>,
// ) -> ContractResult<Asset> {
//     simulate_smart_swap_exact_asset_in(deps, ask_denom, routes)
// }
//
// fn query_simulate_smart_swap_exact_asset_in_with_metadata(
//     deps: Deps,
//     asset_in: Asset,
//     ask_denom: String,
//     routes: Vec<Route>,
//     include_spot_price: bool,
// ) -> ContractResult<SimulateSmartSwapExactAssetInResponse> {
//     let asset_out = simulate_smart_swap_exact_asset_in(deps, ask_denom, routes.clone())?;
//
//     let mut response = SimulateSmartSwapExactAssetInResponse {
//         asset_out,
//         spot_price: None,
//     };
//
//     if include_spot_price {
//         response.spot_price = Some(calculate_weighted_spot_price(
//             &PoolmanagerQuerier::new(&deps.querier),
//             asset_in,
//             routes,
//         )?)
//     }
//
//     Ok(response)
// }
//
// fn simulate_smart_swap_exact_asset_in(
//     deps: Deps,
//     ask_denom: String,
//     routes: Vec<Route>,
// ) -> ContractResult<Asset> {
//     let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());
//
//     for route in &routes {
//         let route_asset_out = query_simulate_swap_exact_asset_in(
//             deps,
//             route.offer_asset.clone(),
//             route.operations.clone(),
//         )?;
//
//         asset_out.add(route_asset_out.amount())?;
//     }
//
//     Ok(asset_out)
// }
//
// // Queries the osmosis poolmanager module to simulate a swap exact amount in with metadata
// fn query_simulate_swap_exact_asset_in_with_metadata(
//     deps: Deps,
//     asset_in: Asset,
//     swap_operations: Vec<SwapOperation>,
//     include_spot_price: bool,
// ) -> ContractResult<SimulateSwapExactAssetInResponse> {
//     let mut response = SimulateSwapExactAssetInResponse {
//         asset_out: query_simulate_swap_exact_asset_in(deps, asset_in, swap_operations.clone())?,
//         spot_price: None,
//     };
//
//     if include_spot_price {
//         response.spot_price = Some(calculate_spot_price(
//             &PoolmanagerQuerier::new(&deps.querier),
//             swap_operations,
//         )?)
//     }
//
//     Ok(response)
// }
//
// // Queries the osmosis poolmanager module to simulate a swap exact amount out with metadata
// fn query_simulate_swap_exact_asset_out_with_metadata(
//     deps: Deps,
//     asset_out: Asset,
//     swap_operations: Vec<SwapOperation>,
//     include_spot_price: bool,
// ) -> ContractResult<SimulateSwapExactAssetOutResponse> {
//     let mut response = SimulateSwapExactAssetOutResponse {
//         asset_in: query_simulate_swap_exact_asset_out(deps, asset_out, swap_operations.clone())?,
//         spot_price: None,
//     };
//
//     if include_spot_price {
//         response.spot_price = Some(calculate_spot_price(
//             &PoolmanagerQuerier::new(&deps.querier),
//             swap_operations,
//         )?)
//     }
//
//     Ok(response)
// }
//
// fn calculate_weighted_spot_price(
//     querier: &PoolmanagerQuerier<Empty>,
//     asset_in: Asset,
//     routes: Vec<Route>,
// ) -> ContractResult<Decimal> {
//     let spot_price = routes.into_iter().try_fold(
//         Decimal::zero(),
//         |curr_spot_price, route| -> ContractResult<Decimal> {
//             let route_spot_price = calculate_spot_price(querier, route.operations)?;
//
//             let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());
//
//             Ok(curr_spot_price + (route_spot_price * weight))
//         },
//     )?;
//
//     Ok(spot_price)
// }
//
// // Calculates the spot price for a given vector of swap operations
// fn calculate_spot_price(
//     querier: &PoolmanagerQuerier<Empty>,
//     swap_operations: Vec<SwapOperation>,
// ) -> ContractResult<Decimal> {
//     let spot_price = swap_operations.into_iter().try_fold(
//         Decimal::one(),
//         |curr_spot_price, swap_op| -> ContractResult<Decimal> {
//             let spot_price_res: SpotPriceResponse = querier.spot_price(
//                 swap_op.pool.parse()?, // should not error since we already parsed it earlier
//                 swap_op.denom_in,
//                 swap_op.denom_out,
//             )?;
//             Ok(curr_spot_price.checked_mul(Decimal::from_str(&spot_price_res.spot_price)?)?)
//         },
//     )?;
//
//     Ok(spot_price)
// }
