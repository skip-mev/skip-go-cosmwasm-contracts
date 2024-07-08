use std::collections::VecDeque;
use std::str::FromStr;

use cosmwasm_std::{
    Addr, Binary, Coin, Decimal, Deps, DepsMut, entry_point, Env, MessageInfo, Reply,
    Response, SubMsg, SubMsgResponse, SubMsgResult, to_json_binary, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use pryzm_std::types::pryzm::{
    amm::v1::{
        AmmQuerier, MsgBatchSwapResponse, QuerySimulateBatchSwapResponse, QuerySpotPriceResponse,
        SwapStep, SwapType,
    },
    icstaking::v1::{IcstakingQuerier, MsgStakeResponse, QuerySimulateStakeResponse},
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
    consts,
    error::{ContractError, ContractResult},
    reply_id,
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_SWAP_OPERATIONS, IN_PROGRESS_SWAP_SENDER},
    swap::{parse_coin, SwapExecutionStep},
};

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

    // Extract the execution steps from the provided swap operations
    let execution_steps = extract_execution_steps(operations)?;

    // Execute the swap
    return execute_steps(deps, env, info.sender, coin_in, execution_steps);
}

// Iterates over the swap operations and aggregates the operations into execution steps
fn extract_execution_steps(
    operations: Vec<SwapOperation>,
) -> Result<VecDeque<SwapExecutionStep>, ContractError> {
    // Return error if swap operations is empty
    if operations.is_empty() {
        return Err(ContractError::SwapOperationsEmpty);
    }

    // Create a vector to push the steps into
    let mut execution_steps: VecDeque<SwapExecutionStep> = VecDeque::new();

    // Create a vector to keep consecutive AMM operations in order to batch them into a single step
    let mut amm_swap_steps: Vec<SwapStep> = Vec::new();

    // Iterate over the swap operations
    let mut swap_operations_iter = operations.iter();
    while let Some(swap_op) = swap_operations_iter.next() {
        if swap_op.pool.starts_with(consts::ICSTAKING_POOL_PREFIX) {
            // Validate that the icstaking operation is converting an asset to a cAsset,
            // not a cAsset to an asset which is not supported
            if swap_op.denom_in.starts_with(consts::C_ASSET_PREFIX)
                || !swap_op.denom_out.starts_with(consts::C_ASSET_PREFIX)
            {
                return Err(ContractError::InvalidPool {
                    msg: format!(
                        "icstaking swap operation can only convert an asset to cAsset: cannot convert {} to {}",
                        swap_op.denom_in, swap_op.denom_out
                    )
                });
            }

            // If there are AMM swap steps from before, aggregate and push them into the execution steps
            if amm_swap_steps.len() != 0 {
                execution_steps.push_back(SwapExecutionStep::Swap {
                    swap_steps: amm_swap_steps,
                });
                amm_swap_steps = Vec::new();
            }

            // split and validate the pool string
            let split: Vec<&str> = swap_op.pool.split(":").collect();
            if split.len() != 3 {
                return Err(ContractError::InvalidPool {
                    msg: format!(
                        "icstaking pool string must be in the format \"icstaking:<host_chain_id>:<transfer_channel>\": {}",
                        swap_op.pool
                    )
                });
            }

            // Push the staking operation into the execution steps
            execution_steps.push_back(SwapExecutionStep::Stake {
                host_chain_id: split.get(1).unwrap().to_string(),
                transfer_channel: split.get(2).unwrap().to_string(),
            });
        } else if swap_op.pool.starts_with(consts::AMM_POOL_PREFIX) {
            // replace the pool prefix and parse the pool id
            let pool_id = swap_op.pool.replace(consts::AMM_POOL_PREFIX, "");
            if let Ok(pool) = pool_id.parse() {
                // Add the operation to the amm swap steps
                amm_swap_steps.push(SwapStep {
                    pool_id: pool,
                    token_in: swap_op.denom_in.clone(),
                    token_out: swap_op.denom_out.clone(),
                    amount: None,
                });
            } else {
                return Err(ContractError::InvalidPool {
                    msg: format!("invalid amm pool id {}", pool_id),
                });
            }
        } else {
            return Err(ContractError::InvalidPool {
                msg: format!(
                    "pool must be started with \"amm\" or \"icstaking\": {}",
                    swap_op.pool
                ),
            });
        }
    }
    Ok(execution_steps)
}

// Executes the swap of the provided coin using the provided execution steps for the swapper
fn execute_steps(
    deps: DepsMut,
    env: Env,
    swapper: Addr,
    coin_in: Coin,
    execution_steps: VecDeque<SwapExecutionStep>,
) -> ContractResult<Response> {
    // return error if execution_steps is empty
    if execution_steps.is_empty() {
        return Err(ContractError::SwapOperationsEmpty);
    }

    // convert the first execution step to the appropriate cosmos message
    let first_step = execution_steps.front().unwrap();
    let msg = first_step
        .clone()
        .to_cosmos_msg(env.contract.address.to_string(), coin_in)?;

    // If there is only one execution step, create the transfer funds back message since the swap is done is a single step
    if execution_steps.len() == 1 {
        // Create the transfer funds back message
        let return_denom = first_step.clone().get_return_denom()?;
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

    // if there are more than one step, create sub message for the first step
    let sub_msg = match first_step {
        SwapExecutionStep::Swap { .. } => {
            SubMsg::reply_on_success(msg.clone(), reply_id::BATCH_SWAP_REPLY_ID)
        }
        SwapExecutionStep::Stake { .. } => {
            SubMsg::reply_on_success(msg.clone(), reply_id::STAKE_REPLY_ID)
        }
    };

    // store the steps to continue after the current step is executed in the reply entrypoint
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
    // Get the sub message result from the reply
    let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = reply.result else {
        return Err(ContractError::InvalidState {
            msg: "could not get sub message response from reply result".to_string(),
        });
    };

    let coin_in: Coin;
    match reply.id {
        reply_id::BATCH_SWAP_REPLY_ID => {
            // Parse the batch swap response from the sub message
            let resp: MsgBatchSwapResponse = b.try_into().map_err(ContractError::Std).unwrap();
            if resp.amounts_out.len() != 1 {
                return Err(ContractError::InvalidMsgResponse {
                    msg: "unexpected amounts out length is batch swap response".to_string(),
                });
            }
            coin_in = parse_coin(resp.amounts_out.first().unwrap().clone())?
        }
        reply_id::STAKE_REPLY_ID => {
            // Parse the stake response from the sub message
            let resp: MsgStakeResponse = b.try_into().map_err(ContractError::Std).unwrap();
            if let Some(c_amount) = resp.c_amount {
                coin_in = parse_coin(c_amount)?
            } else {
                return Err(ContractError::InvalidMsgResponse {
                    msg: "expected valid c_amount in stake response, received None".to_string(),
                });
            }
        }
        _ => {
            return Err(ContractError::InvalidState {
                msg: format!("unexpected reply id {}", reply.id),
            });
        }
    }

    let mut in_progress_exec_steps = IN_PROGRESS_SWAP_OPERATIONS.load(deps.storage)?;
    IN_PROGRESS_SWAP_OPERATIONS.remove(deps.storage);

    let swapper = IN_PROGRESS_SWAP_SENDER.load(deps.storage)?;
    IN_PROGRESS_SWAP_SENDER.remove(deps.storage);

    in_progress_exec_steps.pop_front();
    execute_steps(deps, env, swapper, coin_in, in_progress_exec_steps)?;

    Ok(Response::new().add_attribute("action", "sub_msg_reply_success"))
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in,
            swap_operations,
        } => to_json_binary(&query_simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => to_json_binary(&query_simulate_swap_exact_asset_out(
            deps,
            asset_out,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in,
            swap_operations,
            include_spot_price,
        } => to_json_binary(&query_simulate_swap_exact_asset_in_with_metadata(
            deps,
            asset_in,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out,
            swap_operations,
            include_spot_price,
        } => to_json_binary(&query_simulate_swap_exact_asset_out_with_metadata(
            deps,
            asset_out,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSmartSwapExactAssetIn { routes, .. } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_json_binary(&query_simulate_smart_swap_exact_asset_in(
                deps, ask_denom, routes,
            )?)
        }
        QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
            routes,
            asset_in,
            include_spot_price,
        } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_json_binary(&query_simulate_smart_swap_exact_asset_in_with_metadata(
                deps,
                asset_in,
                ask_denom,
                routes,
                include_spot_price,
            )?)
        }
    }
    .map_err(From::from)
}

fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin in from asset in, error if asset in is not a
    // native coin because Pryzm does not support CW20 tokens.
    let coin_in = match asset_in {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_in's denom is the same as the first swap operation's denom in
    if coin_in.denom != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    let amm_querier = &AmmQuerier::new(&deps.querier);
    let icstaking_querier = &IcstakingQuerier::new(&deps.querier);

    let execution_steps = extract_execution_steps(swap_operations)?;
    let mut step_amount = coin_in;
    for step in execution_steps {
        match step {
            SwapExecutionStep::Swap { swap_steps } => {
                let mut vec = swap_steps.clone();
                if let Some(first_step) = vec.first_mut() {
                    first_step.amount = step_amount.amount.to_string().into();
                }
                let res: QuerySimulateBatchSwapResponse =
                    amm_querier.simulate_batch_swap(SwapType::GivenIn.into(), vec)?;
                step_amount = parse_coin(res.amounts_out.first().unwrap().clone())?;
            }
            SwapExecutionStep::Stake {
                host_chain_id,
                transfer_channel,
            } => {
                let res: QuerySimulateStakeResponse = icstaking_querier.simulate_stake(
                    host_chain_id,
                    transfer_channel,
                    step_amount.amount.to_string().into(),
                    None,
                )?;
                step_amount = parse_coin(res.amount_out.unwrap())?;
            }
        }
    }

    Ok(Asset::from(step_amount))
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin out from asset out, error if asset out is not a
    // native coin because Osmosis does not support CW20 tokens.
    let coin_out = match asset_out {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    let amm_querier = &AmmQuerier::new(&deps.querier);
    let icstaking_querier = &IcstakingQuerier::new(&deps.querier);

    let mut step_amount = coin_out;
    let execution_steps = extract_execution_steps(swap_operations).unwrap();
    for step in execution_steps.iter().rev() {
        match step {
            SwapExecutionStep::Swap { swap_steps } => {
                let mut vec = swap_steps.clone();
                vec.reverse();
                if let Some(first_step) = vec.last_mut() {
                    first_step.amount = step_amount.amount.to_string().into();
                }
                let res: QuerySimulateBatchSwapResponse =
                    amm_querier.simulate_batch_swap(SwapType::GivenOut.into(), vec)?;
                step_amount = parse_coin(res.amounts_in.first().unwrap().clone())?;
            }
            SwapExecutionStep::Stake {
                host_chain_id,
                transfer_channel,
            } => {
                let res: QuerySimulateStakeResponse = icstaking_querier.simulate_stake(
                    host_chain_id.to_string(),
                    transfer_channel.to_string(),
                    None,
                    step_amount.amount.to_string().into(),
                )?;
                step_amount = parse_coin(res.amount_in.unwrap())?;
            }
        }
    }

    Ok(Asset::from(step_amount))
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    simulate_smart_swap_exact_asset_in(deps, ask_denom, routes)
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<SimulateSmartSwapExactAssetInResponse> {
    let asset_out = simulate_smart_swap_exact_asset_in(deps, ask_denom, routes.clone())?;

    let mut response = SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(calculate_weighted_spot_price(deps, asset_in, routes)?)
    }

    Ok(response)
}

fn simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());

    for route in &routes {
        let route_asset_out = query_simulate_swap_exact_asset_in(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
        )?;

        asset_out.add(route_asset_out.amount())?;
    }

    Ok(asset_out)
}

// Queries the osmosis poolmanager module to simulate a swap exact amount in with metadata
fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    let mut response = SimulateSwapExactAssetInResponse {
        asset_out: query_simulate_swap_exact_asset_in(deps, asset_in, swap_operations.clone())?,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(calculate_spot_price(deps, swap_operations)?)
    }

    Ok(response)
}

fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {
    let mut response = SimulateSwapExactAssetOutResponse {
        asset_in: query_simulate_swap_exact_asset_out(deps, asset_out, swap_operations.clone())?,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(calculate_spot_price(deps, swap_operations)?)
    }

    Ok(response)
}

fn calculate_weighted_spot_price(
    deps: Deps,
    asset_in: Asset,
    routes: Vec<Route>,
) -> ContractResult<Decimal> {
    let spot_price = routes.into_iter().try_fold(
        Decimal::zero(),
        |curr_spot_price, route| -> ContractResult<Decimal> {
            let route_spot_price = calculate_spot_price(deps, route.operations)?;

            let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());

            Ok(curr_spot_price + (route_spot_price * weight))
        },
    )?;

    Ok(spot_price)
}

fn calculate_spot_price(
    deps: Deps,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Decimal> {
    let execution_steps = extract_execution_steps(swap_operations)?;

    let amm_querier = &AmmQuerier::new(&deps.querier);
    let icstaking_querier = &IcstakingQuerier::new(&deps.querier);

    let spot_price = execution_steps.into_iter().try_fold(
        Decimal::one(),
        |curr_spot_price, step| -> ContractResult<Decimal> {
            let step_spot_price = match step {
                SwapExecutionStep::Swap { swap_steps } => swap_steps.into_iter().try_fold(
                    Decimal::one(),
                    |curr_spot_price, step| -> ContractResult<Decimal> {
                        let spot_price_res: QuerySpotPriceResponse = amm_querier.spot_price(
                            step.pool_id,
                            step.token_in,
                            step.token_out,
                            false,
                        )?;
                        Ok(curr_spot_price
                            .checked_mul(Decimal::from_str(&spot_price_res.spot_price)?)?)
                    },
                ),
                SwapExecutionStep::Stake {
                    host_chain_id,
                    transfer_channel,
                } => {
                    let amount =
                        Decimal::from_ratio(Uint128::from(1000000u32), Uint128::from(1u32));
                    let res: QuerySimulateStakeResponse = icstaking_querier.simulate_stake(
                        host_chain_id,
                        transfer_channel,
                        amount.to_string().into(),
                        None,
                    )?;
                    Ok(Decimal::from_str(&res.amount_out.unwrap().amount)
                        .unwrap()
                        .checked_div(amount)
                        .unwrap())
                }
            };

            Ok(curr_spot_price.checked_mul(step_spot_price?)?)
        },
    )?;

    Ok(spot_price)
}
