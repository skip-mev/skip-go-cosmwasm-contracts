use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::swap_baby;
use crate::swap_baby::Hop;
use cosmwasm_std::{
    to_json_binary, to_json_string, Addr, Binary, Coin, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdError, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Item;
use cw_utils::one_coin;
use skip::asset::Asset;
use skip::swap::{
    get_ask_denom_for_routes, ExecuteMsg, QueryMsg, Route, SimulateSmartSwapExactAssetInResponse,
    SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation,
};

/////////////
/// STATE ///
/////////////
const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("e");
const ROUTER_CONTRACT_ADDRESS: Item<Addr> = Item::new("r");

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    Ok(Response::default())
}

///////////////////
/// INSTANTIATE ///
///////////////////

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address and router address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    let checked_router_address = deps.api.addr_validate(&msg.router_contract_address)?;

    // Store the entry point and router contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;
    ROUTER_CONTRACT_ADDRESS.save(deps.storage, &checked_router_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

/////////////////
/// EXECUTE ////
////////////////

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Swap { operations } => execute_swap(deps, env, info, operations),
        _ => unimplemented!("not implemented"),
    }
}

// Executes a swap with the given swap operations and then transfers the funds back to the caller
fn execute_swap(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
) -> Result<Response, ContractError> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get coin in from the message info, error if there is not exactly one coin sent
    // this will fail in case more coins are sent.
    let coin_in = one_coin(&info)?;

    let hops = operations.into_iter().map(|e| swap_baby::Hop {
        pool: e.pool,
        denom: e.denom_out,
    });

    let router_contract_address = ROUTER_CONTRACT_ADDRESS.load(deps.storage)?;

    // Make the swap msg
    let swap_msg = swap_baby::ExecuteMsg::SwapExactAmountInWithHops {
        receiver: Some(info.sender.into()),
        min_out: Uint128::zero(),
        hops: hops.collect(),
    }
    .into_cosmos_msg(router_contract_address, vec![coin_in])?;

    Ok(Response::new()
        .add_message(swap_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => Ok(to_json_binary(&query_simulate_swap_exact_asset_out(
            deps,
            asset_out,
            swap_operations,
        )?)?),
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in,
            swap_operations,
        } => Ok(to_json_binary(&query_simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?)?),
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out,
            swap_operations,
            include_spot_price,
        } => Ok(to_json_binary(
            &query_simulate_swap_exact_asset_out_with_metadata(
                deps,
                asset_out,
                swap_operations,
                include_spot_price,
            )?,
        )?),
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in,
            swap_operations,
            include_spot_price,
        } => Ok(to_json_binary(
            &query_simulate_swap_exact_asset_in_with_metadata(
                deps,
                asset_in,
                swap_operations,
                include_spot_price,
            )?,
        )?),
        QueryMsg::SimulateSmartSwapExactAssetIn { asset_in, routes } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;
            Ok(to_json_binary(&query_simulate_smart_swap_exact_asset_in(
                deps, asset_in, ask_denom, &routes,
            )?)?)
        }
        QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
            asset_in,
            routes,
            include_spot_price,
        } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            Ok(to_json_binary(
                &query_simulate_smart_swap_exact_asset_in_with_metadata(
                    deps,
                    asset_in,
                    ask_denom,
                    routes,
                    include_spot_price,
                )?,
            )?)
        }
    }
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    operations: Vec<SwapOperation>,
) -> Result<Asset, ContractError> {
    query_simulate_swap_exact_asset_out_with_metadata(deps, asset_out, operations, false)
        .map(|r| r.asset_in)
}

fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    mut operations: Vec<SwapOperation>,
    include_price: bool,
) -> Result<SimulateSwapExactAssetOutResponse, ContractError> {
    let coin_out = get_coin_from_asset(asset_out)?;
    let (first_op, last_op) = (
        operations
            .first()
            .ok_or(ContractError::SwapOperationsEmpty)?,
        operations
            .last()
            .ok_or(ContractError::SwapOperationsEmpty)?,
    );

    // check that coin out matches the last swap operations out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    let denom_in = first_op.denom_in.clone();

    let router = ROUTER_CONTRACT_ADDRESS.load(deps.storage)?;

    let first_hop = Hop {
        pool: first_op.pool.clone(),
        denom: first_op.denom_in.clone(),
    };

    let mut hops = vec![first_hop];
    // we omit the last swap operation
    // since that is already the coin_out
    operations.pop();
    // if it is not empty, we fill the other left operations.
    if !operations.is_empty() {
        hops.extend(operations.into_iter().map(|op| Hop {
            pool: op.pool,
            denom: op.denom_out,
        }));
    }

    let query_msg = swap_baby::QueryMsg::SimulateSwapExactAmountOutWithHops {
        want_out: coin_out,
        hops,
    };

    let router_resp = deps
        .querier
        .query_wasm_smart::<swap_baby::QuerySimulateSwapExactAmountOutWithHopsResponse>(
            router, &query_msg,
        )
        .map_err(|_| {
            let ctx = to_json_string(&query_msg).unwrap();
            StdError::generic_err(ctx)
        })?;

    Ok(SimulateSwapExactAssetOutResponse {
        asset_in: Coin {
            denom: denom_in,
            amount: router_resp.need_input,
        }
        .into(),
        spot_price: include_price.then_some(router_resp.spot_price),
    })
}

fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset: Asset,
    operations: Vec<SwapOperation>,
) -> Result<Asset, ContractError> {
    query_simulate_swap_exact_asset_in_with_metadata(deps, asset, operations, false)
        .map(|v| v.asset_out)
}

fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset: Asset,
    operations: Vec<SwapOperation>,
    price: bool,
) -> Result<SimulateSwapExactAssetInResponse, ContractError> {
    let expected_denom_out = operations.last().unwrap().denom_out.clone();

    let coin = get_coin_from_asset(asset)?;

    let router = ROUTER_CONTRACT_ADDRESS.load(deps.storage)?;

    let query_msg = swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
        input: coin,
        hops: operations
            .into_iter()
            .map(|op| Hop {
                pool: op.pool,
                denom: op.denom_out,
            })
            .collect(),
    };

    let router_resp = deps
        .querier
        .query_wasm_smart::<swap_baby::SwapExactAmountInWithHopsResponse>(router, &query_msg)?;

    let coin_out = Coin {
        denom: expected_denom_out,
        amount: router_resp.coin_out,
    };

    Ok(SimulateSwapExactAssetInResponse {
        asset_out: coin_out.into(),
        spot_price: price.then_some(router_resp.spot_price),
    })
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_price: bool,
) -> Result<SimulateSmartSwapExactAssetInResponse, ContractError> {
    let (asset_out, spot_price) =
        simulate_smart_swap_exact_asset_in(deps, asset_in, ask_denom, &routes, include_price)?;

    Ok(SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price,
    })
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: &[Route],
) -> Result<Asset, ContractError> {
    simulate_smart_swap_exact_asset_in(deps, asset_in, ask_denom, routes, false).map(|r| r.0)
}

fn simulate_smart_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: &[Route],
    include_price: bool,
) -> Result<(Asset, Option<Decimal>), ContractError> {
    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());

    let mut weighted_price = Decimal::zero();

    for route in routes {
        let swap_in_with_meta = query_simulate_swap_exact_asset_in_with_metadata(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
            include_price,
        )?;

        asset_out.add(swap_in_with_meta.asset_out.amount())?;

        if include_price {
            let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());
            // the spot price returned by the swap baby router is the
            // price of the cumulative swap operations.
            weighted_price += swap_in_with_meta.spot_price.unwrap() * weight;
        }
    }

    Ok((asset_out, include_price.then_some(weighted_price)))
}

fn get_coin_from_asset(asset: Asset) -> Result<Coin, ContractError> {
    match asset {
        Asset::Native(coin) => Ok(coin),
        Asset::Cw20(_) => Err(ContractError::AssetNotNative),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {}
}
