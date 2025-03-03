use crate::state::MANTRA_DEX_POOL_MANAGER_ADDRESS;
use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    ensure, entry_point, to_json_binary, wasm_execute, Binary, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, Uint128,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use skip::swap::MantraDexInstantiateMsg;
use skip::{
    asset::Asset,
    swap::{
        get_ask_denom_for_routes, ExecuteMsg, MigrateMsg, QueryMsg, Route,
        SimulateSmartSwapExactAssetInResponse, SimulateSwapExactAssetInResponse,
        SimulateSwapExactAssetOutResponse, SwapOperation,
    },
};

use crate::pool_manager::{
    ExecuteMsg as MantraPoolManagerExecuteMsg, QueryMsg as MantraQueryMsg,
    ReverseSimulationResponse, SimulationResponse, SwapOperation as MantraSwapOperation,
    MAX_ALLOWED_SLIPPAGE,
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
    msg: MantraDexInstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    // Validate entry point contract address
    let checked_mantra_pool_manager_address =
        deps.api.addr_validate(&msg.mantra_pool_manager_address)?;

    // Store MANTRA dex pool manager address
    MANTRA_DEX_POOL_MANAGER_ADDRESS.save(deps.storage, &checked_mantra_pool_manager_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "mantra_pool_manager_address",
            checked_mantra_pool_manager_address.to_string(),
        ))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Swap { operations } => execute_swap(deps, info, operations),
        _ => {
            unimplemented!()
        }
    }
}

fn execute_swap(
    deps: DepsMut,
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

    // sanity check
    ensure!(
        coin_in.amount != Uint128::zero(),
        ContractError::NoOfferAssetAmount
    );

    // Create a response object to return
    let response: Response = Response::new().add_attribute("action", "execute_swap");

    // map SwapOperation into MantraSwapOperation
    let mantra_swap_operations: Vec<MantraSwapOperation> = operations
        .iter()
        .map(|op| MantraSwapOperation::MantraSwap {
            token_in_denom: op.denom_in.clone(),
            token_out_denom: op.denom_out.clone(),
            pool_identifier: op.pool.clone(),
        })
        .collect();

    ensure!(
        !mantra_swap_operations.is_empty(),
        ContractError::SwapOperationsEmpty
    );

    let msg = MantraPoolManagerExecuteMsg::ExecuteSwapOperations {
        operations: mantra_swap_operations,
        minimum_receive: None,
        receiver: Some(entry_point_contract_address.to_string()),
        max_spread: Some(MAX_ALLOWED_SLIPPAGE.parse::<Decimal>()?),
    };

    // Create swap message on MANTRA dex pool manager
    let mantra_dex_pool_manager = MANTRA_DEX_POOL_MANAGER_ADDRESS.load(deps.storage)?;

    Ok(response
        .add_message(wasm_execute(mantra_dex_pool_manager, &msg, vec![coin_in])?)
        .add_attribute("action", "swap"))
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

/// Queries the MANTRA dex pool manager to simulate a swap exact amount in
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

/// Queries the MANTRA dex pool manager to simulate a multi-hop swap exact amount out
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

/// Queries the MANTRA dex pool manager to simulate a swap exact amount in with metadata
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

/// Queries the MANTRA dex pool manager to simulate a multi-hop swap exact amount out with metadata
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

/// Simulates a swap exact amount in request, returning the asset out and optionally the reverse simulation responses
fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<SimulationResponse>)> {
    let mantra_pool_manager = MANTRA_DEX_POOL_MANAGER_ADDRESS.load(deps.storage)?;

    let (asset_out, responses) = swap_operations.iter().try_fold(
        (asset_in, Vec::new()),
        |(asset_out, mut responses), operation| -> Result<_, ContractError> {
            let offer_asset = match asset_out {
                Asset::Native(coin) => coin,
                Asset::Cw20(_) => unimplemented!("CW20 not supported"),
            };

            // Query mantra's pool manager to get the simulation response
            let res: SimulationResponse = deps.querier.query_wasm_smart(
                &mantra_pool_manager,
                &MantraQueryMsg::Simulation {
                    offer_asset: offer_asset.clone(),
                    ask_asset_denom: operation.denom_out.clone(),
                    pool_identifier: operation.pool.clone(),
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

/// Simulates a swap exact amount out request, returning the asset in needed and optionally the reverse simulation responses
fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<ReverseSimulationResponse>)> {
    let mantra_pool_manager = MANTRA_DEX_POOL_MANAGER_ADDRESS.load(deps.storage)?;

    let (asset_in, responses) = swap_operations.iter().rev().try_fold(
        (asset_out, Vec::new()),
        |(asset_in_needed, mut responses), operation| -> Result<_, ContractError> {
            let ask_asset = match asset_in_needed {
                Asset::Native(coin) => coin,
                Asset::Cw20(_) => unimplemented!("CW20 not supported"),
            };

            // Query the mantra's pool manager to get the reverse simulation response
            let res: ReverseSimulationResponse = deps.querier.query_wasm_smart(
                &mantra_pool_manager,
                &MantraQueryMsg::ReverseSimulation {
                    ask_asset: ask_asset.clone(),
                    offer_asset_denom: operation.denom_in.to_string(),
                    pool_identifier: operation.pool.to_string(),
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

/// Calculates the spot price using simulation responses
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
                .checked_add(res.burn_fee_amount)?
                .checked_add(res.extra_fees_amount)?;

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

/// Calculates the spot price using reverse simulaation responses
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
                    .checked_add(res.burn_fee_amount)?
                    .checked_add(res.extra_fees_amount)?;

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
