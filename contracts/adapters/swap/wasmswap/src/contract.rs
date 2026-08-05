use crate::{
    error::{ContractError, ContractResult},
    pool::{
        get_output_price, FeeResponse, InfoResponse, PoolExecuteMsg, PoolQueryMsg,
        Token1ForToken2PriceResponse, Token2ForToken1PriceResponse, TokenSelect,
    },
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_utils::one_coin;
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

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    Ok(Response::new().add_attribute("action", "migrate"))
}

///////////////////
/// INSTANTIATE ///
///////////////////

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

// cw20 entry point: the token is sent to this contract via Send, then swapped.
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

    // Treat the originator of the send as the sender; it is validated below to be
    // the entry point contract address.
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
        ExecuteMsg::WasmSwapPoolSwap { operation } => {
            execute_wasmswap_pool_swap(deps, env, info, operation)
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

    // Only the entry point contract may call the adapter.
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    let mut response: Response = Response::new().add_attribute("action", "execute_swap");

    // Each hop is a separate self-call so that at swap time the contract knows
    // the actual balance produced by the previous hop.
    for operation in &operations {
        let swap_msg = WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::WasmSwapPoolSwap {
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

fn execute_wasmswap_pool_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
) -> ContractResult<Response> {
    // Self-call only, external callers are rejected.
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    let offer_asset = get_current_asset_available(&deps, &env, &operation.denom_in)?;

    if offer_asset.amount().is_zero() {
        return Err(ContractError::NoOfferAssetAmount);
    }

    let input_token = input_token_select(&deps.querier, &operation.pool, &operation.denom_in)?;

    // Slippage is enforced by the entry point over the whole route,
    // so the per-hop minimum is set to the smallest possible value.
    let swap_msg = to_json_binary(&PoolExecuteMsg::Swap {
        input_token,
        input_amount: offer_asset.amount(),
        min_output: Uint128::one(),
        expiration: None,
    })?;

    let msgs: Vec<CosmosMsg> = match &offer_asset {
        Asset::Native(coin) => vec![WasmMsg::Execute {
            contract_addr: operation.pool.clone(),
            msg: swap_msg,
            funds: vec![coin.clone()],
        }
        .into()],
        // wasmswap has no cw20 Send hook: the pool does a TransferFrom itself,
        // so an allowance for the exact amount is granted first.
        Asset::Cw20(coin) => vec![
            WasmMsg::Execute {
                contract_addr: coin.address.clone(),
                msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: operation.pool.clone(),
                    amount: coin.amount,
                    expires: None,
                })?,
                funds: vec![],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: operation.pool.clone(),
                msg: swap_msg,
                funds: vec![],
            }
            .into(),
        ],
    };

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "dispatch_wasmswap_pool_swap"))
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
        QueryMsg::SimulateSmartSwapExactAssetIn { asset_in, routes } => to_json_binary(
            &query_simulate_smart_swap_exact_asset_in(deps, asset_in, routes)?,
        ),
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
        QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
            asset_in,
            routes,
            include_spot_price,
        } => to_json_binary(&query_simulate_smart_swap_exact_asset_in_with_metadata(
            deps,
            asset_in,
            routes,
            include_spot_price,
        )?),
    }
    .map_err(From::from)
}

fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let (asset_out, _) = simulate_swap_exact_asset_in(deps, asset_in, swap_operations, false)?;
    Ok(asset_out)
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let (asset_in, _) = simulate_swap_exact_asset_out(deps, asset_out, swap_operations, false)?;
    Ok(asset_in)
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    let (asset_out, _) = simulate_smart_swap_exact_asset_in(deps, asset_in, routes, false)?;
    Ok(asset_out)
}

fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    let (asset_out, spot_price) =
        simulate_swap_exact_asset_in(deps, asset_in, swap_operations, include_spot_price)?;
    Ok(SimulateSwapExactAssetInResponse {
        asset_out,
        spot_price,
    })
}

fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {
    let (asset_in, spot_price) =
        simulate_swap_exact_asset_out(deps, asset_out, swap_operations, include_spot_price)?;
    Ok(SimulateSwapExactAssetOutResponse {
        asset_in,
        spot_price,
    })
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<SimulateSmartSwapExactAssetInResponse> {
    let (asset_out, spot_price) =
        simulate_smart_swap_exact_asset_in(deps, asset_in, routes, include_spot_price)?;
    Ok(SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price,
    })
}

////////////////////////
/// HELPERS          ///
////////////////////////

/// Which of the pool's two tokens is the input one.
fn input_token_select(
    querier: &QuerierWrapper,
    pool: &str,
    denom_in: &str,
) -> ContractResult<TokenSelect> {
    let info: InfoResponse = querier.query_wasm_smart(pool, &PoolQueryMsg::Info {})?;

    if info.token1_denom.matches(denom_in) {
        Ok(TokenSelect::Token1)
    } else if info.token2_denom.matches(denom_in) {
        Ok(TokenSelect::Token2)
    } else {
        Err(ContractError::PoolDenomMismatch)
    }
}

/// Pool reserves ordered as (input, output) for the given direction.
fn reserves_for_direction(
    querier: &QuerierWrapper,
    pool: &str,
    denom_in: &str,
) -> ContractResult<(Uint128, Uint128)> {
    let info: InfoResponse = querier.query_wasm_smart(pool, &PoolQueryMsg::Info {})?;

    let (r_in, r_out) = if info.token1_denom.matches(denom_in) {
        (info.token1_reserve, info.token2_reserve)
    } else if info.token2_denom.matches(denom_in) {
        (info.token2_reserve, info.token1_reserve)
    } else {
        return Err(ContractError::PoolDenomMismatch);
    };

    if r_in.is_zero() || r_out.is_zero() {
        return Err(ContractError::PoolEmpty);
    }
    Ok((r_in, r_out))
}

fn pool_fee_bps(querier: &QuerierWrapper, pool: &str) -> ContractResult<u128> {
    let fee: FeeResponse = querier.query_wasm_smart(pool, &PoolQueryMsg::Fee {})?;
    Ok(fee.total_fee_bps()?)
}

/// Spot price of a single hop: output per unit of input, fee included.
fn spot_price_for_op(querier: &QuerierWrapper, op: &SwapOperation) -> ContractResult<Decimal> {
    let (r_in, r_out) = reserves_for_direction(querier, &op.pool, &op.denom_in)?;
    let fee_bps = pool_fee_bps(querier, &op.pool)?;

    let price = Decimal::from_ratio(r_out, r_in);
    let fee_multiplier = Decimal::from_ratio(10_000u128 - fee_bps, 10_000u128);
    Ok(price * fee_multiplier)
}

fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<(Asset, Option<Decimal>)> {
    let (last_denom_out, mut amount) = match swap_operations.last() {
        Some(last_op) => (last_op.denom_out.clone(), asset_in.amount()),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // Walk the route asking each pool for its own price, so both fee and
    // slippage are accounted for exactly as they would be on execution.
    for op in &swap_operations {
        let select = input_token_select(&deps.querier, &op.pool, &op.denom_in)?;
        amount = match select {
            TokenSelect::Token1 => {
                let res: Token1ForToken2PriceResponse = deps.querier.query_wasm_smart(
                    &op.pool,
                    &PoolQueryMsg::Token1ForToken2Price {
                        token1_amount: amount,
                    },
                )?;
                res.token2_amount
            }
            TokenSelect::Token2 => {
                let res: Token2ForToken1PriceResponse = deps.querier.query_wasm_smart(
                    &op.pool,
                    &PoolQueryMsg::Token2ForToken1Price {
                        token2_amount: amount,
                    },
                )?;
                res.token1_amount
            }
        };
    }

    let asset_out = Asset::new(deps.api, &last_denom_out, amount);

    let spot_price = if include_spot_price {
        Some(calculate_spot_price(&deps.querier, &swap_operations)?)
    } else {
        None
    };

    Ok((asset_out, spot_price))
}

fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<(Asset, Option<Decimal>)> {
    let first_denom_in = match swap_operations.first() {
        Some(first_op) => first_op.denom_in.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // wasmswap has no reverse price query, so it is computed from reserves and
    // fee while walking the route backwards.
    let mut amount = asset_out.amount();
    for op in swap_operations.iter().rev() {
        let (r_in, r_out) = reserves_for_direction(&deps.querier, &op.pool, &op.denom_in)?;
        let fee_bps = pool_fee_bps(&deps.querier, &op.pool)?;

        if amount >= r_out {
            return Err(ContractError::AssetOutExceedsReserve);
        }
        amount = get_output_price(amount, r_in, r_out, fee_bps)?;
    }

    let asset_in = Asset::new(deps.api, &first_denom_in, amount);

    let spot_price = if include_spot_price {
        Some(calculate_spot_price(&deps.querier, &swap_operations)?)
    } else {
        None
    };

    Ok((asset_in, spot_price))
}

fn simulate_smart_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<(Asset, Option<Decimal>)> {
    let ask_denom = get_ask_denom_for_routes(&routes)?;

    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());
    let mut weighted_price = Decimal::zero();

    for route in &routes {
        let (route_asset_out, _) = simulate_swap_exact_asset_in(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
            false,
        )?;
        asset_out.add(route_asset_out.amount())?;

        if include_spot_price {
            let route_price = calculate_spot_price(&deps.querier, &route.operations)?;
            let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());
            weighted_price += route_price * weight;
        }
    }

    let spot_price = if include_spot_price {
        Some(weighted_price)
    } else {
        None
    };

    Ok((asset_out, spot_price))
}

/// The route's spot price is the product of the per-hop spot prices.
fn calculate_spot_price(
    querier: &QuerierWrapper,
    swap_operations: &[SwapOperation],
) -> ContractResult<Decimal> {
    swap_operations
        .iter()
        .try_fold(Decimal::one(), |acc, op| -> ContractResult<Decimal> {
            Ok(acc * spot_price_for_op(querier, op)?)
        })
}
