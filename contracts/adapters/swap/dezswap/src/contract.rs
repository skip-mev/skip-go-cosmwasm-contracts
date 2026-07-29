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
use dezswap::pair::{
    Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as PairExecuteMsg, QueryMsg as PairQueryMsg,
    ReverseSimulationResponse, SimulationResponse,
};
use skip::{
    asset::{get_current_asset_available, Asset},
    swap::{
        execute_transfer_funds_back, get_ask_denom_for_routes, Cw20HookMsg, ExecuteMsg,
        InstantiateMsg, MigrateMsg, QueryMsg, Route, SimulateSmartSwapExactAssetInResponse,
        SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Dezswap does not export this constant (unlike astroport/oroswap), so it is
// pinned locally to the same value as Astroport 2.9's MAX_ALLOWED_SLIPPAGE.
const MAX_ALLOWED_SLIPPAGE: &str = "0.5";

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

/////////////////
// INSTANTIATE //
/////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

/////////////
// RECEIVE //
/////////////

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
        ExecuteMsg::DezswapPoolSwap { operation } => {
            execute_dezswap_pool_swap(deps, env, info, operation)
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
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    let mut response: Response = Response::new().add_attribute("action", "execute_swap");

    for operation in &operations {
        let swap_msg = WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::DezswapPoolSwap {
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

fn execute_dezswap_pool_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
) -> ContractResult<Response> {
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    let offer_asset = get_current_asset_available(&deps, &env, &operation.denom_in)?;

    if offer_asset.amount().is_zero() {
        return Err(ContractError::NoOfferAssetAmount);
    }

    let msg = match offer_asset {
        Asset::Native(_) => to_json_binary(&PairExecuteMsg::Swap {
            offer_asset: offer_asset.into_dezswap_asset(deps.api)?,
            belief_price: None,
            max_spread: Some(MAX_ALLOWED_SLIPPAGE.parse::<Decimal>()?),
            to: None,
            deadline: None,
        })?,
        Asset::Cw20(_) => to_json_binary(&PairCw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(MAX_ALLOWED_SLIPPAGE.parse::<Decimal>()?),
            to: None,
            deadline: None,
        })?,
    };

    let swap_msg = offer_asset.into_wasm_msg(operation.pool, msg)?;

    Ok(Response::new()
        .add_message(swap_msg)
        .add_attribute("action", "dispatch_dezswap_pool_swap"))
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

fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    if asset_in.denom() != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    let (asset_out, _) = simulate_swap_exact_asset_in(deps, asset_in, swap_operations, false)?;

    Ok(asset_out)
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    if asset_out.denom() != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    let (asset_in, _) = simulate_swap_exact_asset_out(deps, asset_out, swap_operations, false)?;

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

fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    let Some(first_op) = swap_operations.first() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    if asset_in.denom() != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    let mut include_sim_resps = false;
    if include_spot_price {
        include_sim_resps = true;
    }

    let (asset_out, sim_resps) = simulate_swap_exact_asset_in(
        deps,
        asset_in.clone(),
        swap_operations.clone(),
        include_sim_resps,
    )?;

    let mut response = SimulateSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

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

fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    if asset_out.denom() != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    let mut include_sim_resps = false;
    if include_spot_price {
        include_sim_resps = true;
    }

    let (asset_in, sim_resps) = simulate_swap_exact_asset_out(
        deps,
        asset_out.clone(),
        swap_operations.clone(),
        include_sim_resps,
    )?;

    let mut response = SimulateSwapExactAssetOutResponse {
        asset_in,
        spot_price: None,
    };

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

fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<SimulationResponse>)> {
    let (asset_out, responses) = swap_operations.iter().try_fold(
        (asset_in, Vec::new()),
        |(asset_out, mut responses), operation| -> Result<_, ContractError> {
            let dezswap_offer_asset = asset_out.into_dezswap_asset(deps.api)?;

            let res: SimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::Simulation {
                    offer_asset: dezswap_offer_asset,
                },
            )?;

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

fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<ReverseSimulationResponse>)> {
    let (asset_in, responses) = swap_operations.iter().rev().try_fold(
        (asset_out, Vec::new()),
        |(asset_in_needed, mut responses), operation| -> Result<_, ContractError> {
            let dezswap_ask_asset = asset_in_needed.into_dezswap_asset(deps.api)?;

            let res: ReverseSimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::ReverseSimulation {
                    ask_asset: dezswap_ask_asset,
                },
            )?;

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

fn calculate_spot_price_from_simulation_responses(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    simulation_responses: Vec<SimulationResponse>,
) -> ContractResult<Decimal> {
    let (_, spot_price) = swap_operations.iter().zip(simulation_responses).try_fold(
        (asset_in, Decimal::one()),
        |(asset_out, curr_spot_price), (op, res)| -> Result<_, ContractError> {
            let amount_out_without_slippage = res
                .return_amount
                .checked_add(res.spread_amount)?
                .checked_add(res.commission_amount)?;

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
                    .checked_add(res.commission_amount)?;

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
