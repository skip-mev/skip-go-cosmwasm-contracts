use crate::{
    error::{ContractError, ContractResult},
    state::{ASTROVAULT_CASHBACK_ADDRESS, ASTROVAULT_ROUTER_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS},
};
use astrovault::router::{
    self,
    handle_msg::RouterReceiveMsg,
    query_msg::{ConfigResponse, QueryRouteSwapSimulation, RoutePoolType},
    state::HopV2,
};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, QueryRequest, Response, Uint128, WasmMsg, WasmQuery,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw20::{Cw20Coin, Cw20Contract, Cw20ReceiveMsg};
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    error::SkipError,
    swap::{
        get_ask_denom_for_routes, AstrovaultAdapterInstantiateMsg, Cw20HookMsg, ExecuteMsg,
        MigrateMsg, QueryMsg, Route, SimulateSmartSwapExactAssetInResponse,
        SimulateSwapExactAssetInResponse, SwapOperation,
    },
};

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: AstrovaultAdapterInstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    let astrovault_router_contract_address = deps
        .api
        .addr_validate(&msg.astrovault_router_contract_address)?;
    ASTROVAULT_ROUTER_ADDRESS.save(deps.storage, &astrovault_router_contract_address)?;

    // query router configs to get cashback address if available
    let router_config: ConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: astrovault_router_contract_address.to_string(),
            msg: to_json_binary(&router::query_msg::QueryMsg::Config {})?,
        }))?;

    if let Some(cashback) = router_config.cashback {
        // this is needed so the grvt8 won by the swaps executed by this adapter can be sent back to the router address
        ASTROVAULT_CASHBACK_ADDRESS.save(deps.storage, &deps.api.addr_validate(&cashback)?)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "astrovault_router_contract_address",
            astrovault_router_contract_address.to_string(),
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
        Cw20HookMsg::Swap { operations } => {
            execute_swap(deps, env, info, sent_asset.amount(), operations)
        }
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
            let coin = one_coin(&info)?;
            execute_swap(deps, env, info, coin.amount, operations)
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
        _ => {
            unimplemented!()
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount_in: Uint128,
    operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;
    let astrovault_router_contract_address = ASTROVAULT_ROUTER_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    let hops = convert_operations_to_hops(
        deps.as_ref(),
        astrovault_router_contract_address.to_string(),
        operations.clone(),
    )?;

    // Create base astrovault router wasm message
    let router_execute_msg = RouterReceiveMsg::RouteV2 {
        hops,
        minimum_receive: None,
        to: None,
    };

    let initial_asset = Asset::new(deps.api, &operations.first().unwrap().denom_in, amount_in);

    // depending if the initial asset is native or cw20, we set the respective msg
    let astrovault_router_wasm_msg = match initial_asset {
        Asset::Native(native_asset) => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: astrovault_router_contract_address.to_string(),
            funds: vec![native_asset],
            msg: to_json_binary(&router::handle_msg::ExecuteMsg::Receive(
                cw20::Cw20ReceiveMsg {
                    sender: env.contract.address.to_string(),
                    amount: amount_in,
                    msg: to_json_binary(&router_execute_msg)?,
                },
            ))?,
        }),
        Asset::Cw20(cw20_asset) => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_asset.address.to_string(),
            funds: vec![],
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::Send {
                contract: astrovault_router_contract_address.to_string(),
                amount: cw20_asset.amount,
                msg: to_json_binary(&router_execute_msg)?,
            })?,
        }),
    };

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

    Ok(Response::new()
        .add_attribute("action", "execute_swap")
        .add_message(astrovault_router_wasm_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swaps_and_transfer_back"))
}

pub fn execute_transfer_funds_back(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swapper: Addr,
    return_denom: String,
) -> Result<Response, SkipError> {
    // Ensure the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(SkipError::Unauthorized);
    }

    // Create the transfer funds back message
    let transfer_funds_back_msg: CosmosMsg = match deps.api.addr_validate(&return_denom) {
        Ok(contract_addr) => Asset::new(
            deps.api,
            contract_addr.as_str(),
            Cw20Contract(contract_addr.clone()).balance(&deps.querier, &env.contract.address)?,
        )
        .transfer(swapper.as_str()),
        Err(_) => CosmosMsg::Bank(BankMsg::Send {
            to_address: swapper.to_string(),
            amount: deps
                .querier
                .query_all_balances(env.contract.address.clone())?,
        }),
    };

    let mut msgs = vec![transfer_funds_back_msg];

    // ADDED: Also create the return of cashback funds msg if available
    if let Some(cashback_addr) = ASTROVAULT_CASHBACK_ADDRESS.may_load(deps.storage)? {
        if return_denom != cashback_addr {
            msgs.push(
                Asset::new(
                    deps.api,
                    cashback_addr.as_str(),
                    Cw20Contract(cashback_addr.clone())
                        .balance(&deps.querier, &env.contract.address)?,
                )
                .transfer(ASTROVAULT_ROUTER_ADDRESS.load(deps.storage)?.as_str()),
            )
        }
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "dispatch_transfer_funds_back_bank_send"))
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
        _ => {
            unimplemented!()
        }
    }
    .map_err(From::from)
}

// Queries the astrovault pool contracts to simulate a swap exact amount in
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

    let (asset_out, _) = simulate_swap_exact_asset_in(deps, asset_in, swap_operations)?;

    Ok(asset_out)
}

fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<(Asset, Decimal)> {
    let astrovault_router_contract_address = ASTROVAULT_ROUTER_ADDRESS.load(deps.storage)?;
    let hops = convert_operations_to_hops(
        deps,
        astrovault_router_contract_address.to_string(),
        swap_operations,
    )?;

    let simulation_response: QueryRouteSwapSimulation =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: astrovault_router_contract_address.to_string(),
            msg: to_json_binary(&router::query_msg::QueryMsg::RouteSwapSimulation {
                amount: asset_in.amount(),
                hops,
            })?,
        }))?;

    Ok((
        Asset::new(
            deps.api,
            &simulation_response.to.info.to_string(),
            simulation_response.to.amount,
        ),
        simulation_response.to_spot_price,
    ))
}

// Queries the astrovault pool contracts to simulate a swap exact amount in with metadata
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

    let (asset_out, spot_price) = simulate_swap_exact_asset_in(deps, asset_in, swap_operations)?;

    // Create the response
    let response = SimulateSwapExactAssetInResponse {
        asset_out,
        spot_price: if include_spot_price {
            Some(spot_price)
        } else {
            None
        },
    };

    Ok(response)
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    let (asset_out, _) = simulate_smart_swap_exact_asset_in(deps, ask_denom, routes)?;

    Ok(asset_out)
}

fn simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<(Asset, Vec<Decimal>)> {
    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());
    let mut spot_prices = Vec::new();

    for route in &routes {
        let (route_asset_out, spot_price) = simulate_swap_exact_asset_in(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
        )?;

        asset_out.add(route_asset_out.amount())?;
        spot_prices.push(spot_price);
    }

    Ok((asset_out, spot_prices))
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<SimulateSmartSwapExactAssetInResponse> {
    let (asset_out, spot_prices) =
        simulate_smart_swap_exact_asset_in(deps, ask_denom, routes.clone())?;

    let mut response = SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    if include_spot_price {
        let mut spot_price = Decimal::zero();
        for (i, route) in routes.iter().enumerate() {
            let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());
            let route_spot_price = spot_prices[i];
            spot_price += weight * route_spot_price;
        }

        response.spot_price = Some(spot_price);
    }

    Ok(response)
}

pub fn convert_operations_to_hops(
    deps: Deps,
    astrovault_router_contract_address: String,
    operations: Vec<SwapOperation>,
) -> ContractResult<Vec<HopV2>> {
    // Create hops for astrovault router
    let mut hops = vec![];

    // Get Pool types for each operation
    let route_pools_type: Vec<RoutePoolType> =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: astrovault_router_contract_address,
            msg: to_json_binary(&router::query_msg::QueryMsg::RoutePoolsType {
                route_pools_addr: operations.clone().into_iter().map(|op| op.pool).collect(),
            })?,
        }))?;

    for (i, operation) in operations.iter().enumerate() {
        // depending on the pool type of the operation, we add the respective hop type to the astrovault router swap msg
        let pool = astrovault::assets::pools::PoolInfoInput::Addr(operation.pool.clone());
        let from_asset_index = route_pools_type[i]
            .pool_asset_infos
            .iter()
            .position(|x| x.to_string() == operation.denom_in);

        let from_asset_index = match from_asset_index {
            Some(index) => index as u32,
            None => return Err(ContractError::InvalidPoolAsset),
        };

        match route_pools_type[i].pool_type.as_str() {
            "hybrid" => {
                hops.push(HopV2::RatioHopInfo {
                    pool,
                    from_asset_index,
                });
            }
            "standard" => {
                hops.push(HopV2::StandardHopInfo {
                    pool,
                    from_asset_index,
                });
            }
            "stable" => {
                // this type of pool can have more than 2 assets, so we need to find the to_asset_index
                let to_asset_index = route_pools_type[i]
                    .pool_asset_infos
                    .iter()
                    .position(|x| x.to_string() == operation.denom_out);

                let to_asset_index = match to_asset_index {
                    Some(index) => index as u32,
                    None => return Err(ContractError::InvalidPoolAsset),
                };

                hops.push(HopV2::StableHopInfo {
                    pool,
                    from_asset_index,
                    to_asset_index,
                });
            }
            _ => {
                return Err(ContractError::InvalidPoolType);
            }
        }
    }

    Ok(hops)
}
