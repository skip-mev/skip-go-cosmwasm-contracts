use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw_utils::one_coin;
use skip::{
    asset::{get_current_asset_available, Asset},
    swap::{
        execute_transfer_funds_back, get_ask_denom_for_routes, Cw20HookMsg, ExecuteMsg,
        InstantiateMsg, MigrateMsg, QueryMsg, Route, SimulateSmartSwapExactAssetInResponse,
        SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation,
    },
};
use white_whale_std::pool_network::{
    pair::{
        Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as PairExecuteMsg, QueryMsg as PairQueryMsg,
        ReverseSimulationResponse, SimulationResponse,
    },
    swap::MAX_ALLOWED_SLIPPAGE,
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
/// RECEIVE ///
///////////////

// Receive is the main entry point for the contract to
// receive cw20 tokens and execute the swap
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    mut info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> ContractResult<Response> {
    let sent_asset = Asset::Cw20(Cw20Coin {
        address: info.sender.to_string(),
        amount: cw20_msg.amount,
    });
    sent_asset.validate(&deps, &env, &info)?;

    // Set the sender to the originating address that triggered the cw20 send call
    // This is later validated / enforced to be the entry point contract address
    info.sender = deps.api.addr_validate(&cw20_msg.sender)?;

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::Swap { operations } => execute_swap(deps, env, info, operations),
    }
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
        ExecuteMsg::Receive(cw20_msg) => receive_cw20(deps, env, info, cw20_msg),
        ExecuteMsg::Swap { operations } => {
            one_coin(&info)?;
            execute_swap(deps, env, info, operations)
        }
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
        ExecuteMsg::WhiteWhalePoolSwap { operation } => {
            execute_white_whale_pool_swap(deps, env, info, operation)
        }
        _ => {
            unimplemented!()
        }
    }
}

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

    // Create a response object to return
    let mut response: Response = Response::new().add_attribute("action", "execute_swap");

    // Add a white whale pool swap message to the response for each swap operation
    for operation in &operations {
        let swap_msg = WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::WhiteWhalePoolSwap {
                operation: operation.clone(),
            })?,
            funds: vec![],
        };
        response = response.add_message(swap_msg);
    }

    let return_denom = match operations.last() {
        Some(last_op) => last_op.denom_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
            swapper: entry_point_contract_address,
            return_denom,
        })?,
        funds: vec![],
    };

    Ok(response
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swaps_and_transfer_back"))
}

fn execute_white_whale_pool_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
) -> ContractResult<Response> {
    // Ensure the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Get the current asset available on contract to swap in
    let offer_asset = get_current_asset_available(&deps, &env, &operation.denom_in)?;

    // Error if the offer asset amount is zero
    if offer_asset.amount().is_zero() {
        return Err(ContractError::NoOfferAssetAmount);
    }

    // Create the whitewhale pool swap msg depending on the offer asset type
    let msg = match offer_asset {
        Asset::Native(_) => to_json_binary(&PairExecuteMsg::Swap {
            offer_asset: offer_asset.into_white_whale_asset(deps.api)?,
            belief_price: None,
            max_spread: Some(MAX_ALLOWED_SLIPPAGE.parse::<Decimal>()?),
            to: None,
        })?,
        Asset::Cw20(_) => to_json_binary(&PairCw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(MAX_ALLOWED_SLIPPAGE.parse::<Decimal>()?),
            to: None,
        })?,
    };

    // Create the wasm white whale pool swap message
    let swap_msg = offer_asset.into_wasm_msg(operation.pool, msg)?;

    Ok(Response::new()
        .add_message(swap_msg)
        .add_attribute("action", "dispatch_white_whale_pool_swap"))
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
            asset_in,
            routes,
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

// Queries the white whale pool contracts to simulate a swap exact amount in
fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure asset_in's denom is the same as the first swap operation's denom in
    if asset_in.denom() != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    let (asset_out, _) = simulate_swap_exact_asset_in(deps, asset_in, swap_operations, false)?;

    // Return the asset out
    Ok(asset_out)
}

// Queries the white whale pool contracts to simulate a multi-hop swap exact amount out
fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure asset_out's denom is the same as the last swap operation's denom out
    if asset_out.denom() != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    let (asset_in, _) = simulate_swap_exact_asset_out(deps, asset_out, swap_operations, false)?;

    // Return the asset in needed
    Ok(asset_in)
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    let (asset_out, _) = simulate_smart_swap_exact_asset_in(deps, ask_denom, routes, false)?;

    Ok(asset_out)
}

// Queries the white whale pool contracts to simulate a swap exact amount in with metadata
fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    // Error if swap operations is empty
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure asset_in's denom is the same as the first swap operation's denom in
    if asset_in.denom() != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // Determine if we should request the simulation responses from simulate_swap_exact_asset_in
    let mut include_sim_resps = false;
    if include_spot_price {
        include_sim_resps = true;
    }

    // Simulate the swap exact amount in
    let (asset_out, sim_resps) = simulate_swap_exact_asset_in(
        deps,
        asset_in.clone(),
        swap_operations.clone(),
        include_sim_resps,
    )?;

    // Create the response
    let mut response = SimulateSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    // Include the spot price in the response if requested
    if include_spot_price {
        response.spot_price = Some(calculate_spot_price_from_simulation_responses(
            deps,
            asset_in,
            swap_operations,
            sim_resps,
        )?)
    }

    Ok(response)
}

// Queries the white whale pool contracts to simulate a multi-hop swap exact amount out with metadata
fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {
    // Error if swap operations is empty
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure asset_out's denom is the same as the last swap operation's denom out
    if asset_out.denom() != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    // Determine if we should request the simulation responses from simulate_swap_exact_asset_out
    let mut include_sim_resps = false;
    if include_spot_price {
        include_sim_resps = true;
    }

    // Simulate the swap exact amount out
    let (asset_in, sim_resps) = simulate_swap_exact_asset_out(
        deps,
        asset_out.clone(),
        swap_operations.clone(),
        include_sim_resps,
    )?;

    // Create the response
    let mut response = SimulateSwapExactAssetOutResponse {
        asset_in,
        spot_price: None,
    };

    // Include the spot price in the response if requested
    if include_spot_price {
        response.spot_price = Some(calculate_spot_price_from_reverse_simulation_responses(
            deps,
            asset_out,
            swap_operations,
            sim_resps,
        )?)
    }

    Ok(response)
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<SimulateSmartSwapExactAssetInResponse> {
    let (asset_out, simulation_responses) =
        simulate_smart_swap_exact_asset_in(deps, ask_denom, routes.clone(), include_spot_price)?;

    let mut response = SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(calculate_weighted_spot_price_from_simulation_responses(
            deps,
            asset_in,
            routes,
            simulation_responses,
        )?)
    }

    Ok(response)
}

fn assert_max_spread(return_amount: Uint128, spread_amount: Uint128) -> ContractResult<()> {
    let max_spread = MAX_ALLOWED_SLIPPAGE.parse::<Decimal>()?;
    if Decimal::from_ratio(spread_amount, return_amount + spread_amount) > max_spread {
        return Err(ContractError::MaxSpreadAssertion {});
    }
    Ok(())
}

// Simulates a swap exact amount in request, returning the asset out and optionally the reverse simulation responses
fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<SimulationResponse>)> {
    let (asset_out, responses) = swap_operations.iter().try_fold(
        (asset_in, Vec::new()),
        |(asset_out, mut responses), operation| -> Result<_, ContractError> {
            // Get the white whale offer asset type
            let white_whale_offer_asset = asset_out.into_white_whale_asset(deps.api)?;

            // Query the white whale pool contract to get the simulation response
            let res: SimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::Simulation {
                    offer_asset: white_whale_offer_asset,
                },
            )?;

            // Assert the operation does not exceed the max spread limit
            assert_max_spread(res.return_amount, res.spread_amount)?;

            if include_responses {
                responses.push(res.clone());
            }

            Ok((
                Asset::new(deps.api, &operation.denom_out, res.return_amount),
                responses,
            ))
        },
    )?;

    Ok((asset_out, responses))
}

// Simulates a swap exact amount out request, returning the asset in needed and optionally the reverse simulation responses
fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<ReverseSimulationResponse>)> {
    let (asset_in, responses) = swap_operations.iter().rev().try_fold(
        (asset_out, Vec::new()),
        |(asset_in_needed, mut responses), operation| -> Result<_, ContractError> {
            // Get the white whale ask asset type
            let white_whale_ask_asset = asset_in_needed.into_white_whale_asset(deps.api)?;

            // Query the white whale pool contract to get the reverse simulation response
            let res: ReverseSimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::ReverseSimulation {
                    ask_asset: white_whale_ask_asset,
                },
            )?;

            // Assert the operation does not exceed the max spread limit
            assert_max_spread(res.offer_amount, res.spread_amount)?;

            if include_responses {
                responses.push(res.clone());
            }

            Ok((
                Asset::new(
                    deps.api,
                    &operation.denom_in,
                    res.offer_amount.checked_add(Uint128::one())?,
                ),
                responses,
            ))
        },
    )?;

    Ok((asset_in, responses))
}

fn simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<Vec<SimulationResponse>>)> {
    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());
    let mut simulation_responses = Vec::new();

    for route in &routes {
        let (route_asset_out, route_simulation_responses) = simulate_swap_exact_asset_in(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
            include_responses,
        )?;

        asset_out.add(route_asset_out.amount())?;

        if include_responses {
            simulation_responses.push(route_simulation_responses);
        }
    }

    Ok((asset_out, simulation_responses))
}

fn calculate_weighted_spot_price_from_simulation_responses(
    deps: Deps,
    asset_in: Asset,
    routes: Vec<Route>,
    simulation_responses: Vec<Vec<SimulationResponse>>,
) -> ContractResult<Decimal> {
    let spot_price = routes.into_iter().zip(simulation_responses).try_fold(
        Decimal::zero(),
        |curr_spot_price, (route, res)| -> ContractResult<Decimal> {
            let route_spot_price = calculate_spot_price_from_simulation_responses(
                deps,
                asset_in.clone(),
                route.operations,
                res,
            )?;

            let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());

            Ok(curr_spot_price + (route_spot_price * weight))
        },
    )?;

    Ok(spot_price)
}

// Calculate the spot price using simulation responses
fn calculate_spot_price_from_simulation_responses(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    simulation_responses: Vec<SimulationResponse>,
) -> ContractResult<Decimal> {
    let (_, spot_price) = swap_operations.iter().zip(simulation_responses).try_fold(
        (asset_in, Decimal::one()),
        |(asset_out, curr_spot_price), (op, res)| -> Result<_, ContractError> {
            // Calculate the amount out without slippage
            let amount_out_without_slippage = res
                .return_amount
                .checked_add(res.spread_amount)?
                .checked_add(res.swap_fee_amount)?
                .checked_add(res.protocol_fee_amount)?
                .checked_add(res.burn_fee_amount)?;

            Ok((
                Asset::new(deps.api, &op.denom_out, res.return_amount),
                curr_spot_price.checked_mul(Decimal::from_ratio(
                    amount_out_without_slippage,
                    asset_out.amount(),
                ))?,
            ))
        },
    )?;

    Ok(spot_price)
}

// Calculates the spot price using reverse simulaation responses
fn calculate_spot_price_from_reverse_simulation_responses(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    reverse_simulation_responses: Vec<ReverseSimulationResponse>,
) -> ContractResult<Decimal> {
    let (_, spot_price) = swap_operations
        .iter()
        .rev()
        .zip(reverse_simulation_responses)
        .try_fold(
            (asset_out, Decimal::one()),
            |(asset_in_needed, curr_spot_price), (op, res)| -> Result<_, ContractError> {
                let amount_out_without_slippage = asset_in_needed
                    .amount()
                    .checked_add(res.spread_amount)?
                    .checked_add(res.swap_fee_amount)?
                    .checked_add(res.protocol_fee_amount)?
                    .checked_add(res.burn_fee_amount)?;

                Ok((
                    Asset::new(
                        deps.api,
                        &op.denom_in,
                        res.offer_amount.checked_add(Uint128::one())?,
                    ),
                    curr_spot_price.checked_mul(Decimal::from_ratio(
                        amount_out_without_slippage,
                        res.offer_amount,
                    ))?,
                ))
            },
        )?;

    Ok(spot_price)
}
