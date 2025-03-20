use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use elys_std::types::elys::amm::{AmmQuerier, MsgUpFrontSwapExactAmountIn, QuerySwapEstimationExactAmountOutResponse, QuerySwapEstimationResponse, SwapAmountInRoute, SwapAmountOutRoute};
use skip::{
    asset::Asset,
    proto_coin::ProtoCoin,
    swap::{
        convert_swap_operations, execute_transfer_funds_back, get_ask_denom_for_routes, ExecuteMsg,
        InstantiateMsg, MigrateMsg, QueryMsg, Route, SimulateSmartSwapExactAssetInResponse,
        SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation,
    },
};
use std::str::FromStr;

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    unimplemented!()
}

/////////////////
// INSTANTIATE //
/////////////////

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

    let return_denom = match operations.last() {
        Some(last_op) => last_op.denom_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // Create the Elys Amm swap exact amount in message
    let swap_msg = create_elys_swap_msg(&env, coin_in, operations)?;

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
            swapper: info.sender,
            return_denom,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

//////////////////////
// HELPER FUNCTIONS //
//////////////////////

// Creates the Elys amm swap exact amount in message
fn create_elys_swap_msg(
    env: &Env,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<CosmosMsg> {
    // Convert the swap operations to elys swap amount in routes
    // Return an error if there was an error converting the swap
    // operations to elys swap amount in routes.
    let elys_swap_amount_in_routes: Vec<SwapAmountInRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;


    // Create the elys amm module swap exact amount in message
    // The token out min amount is set to 1 because we are not concerned
    // with the minimum amount in this contract, that gets verified in the
    // entry point contract.
    let swap_msg = MsgUpFrontSwapExactAmountIn {
        sender: env.contract.address.to_string(),
        routes: elys_swap_amount_in_routes,
        token_in: Some(ProtoCoin(coin_in).into()),
        token_out_min_amount: "1".to_string(),
    }.into();

    Ok(swap_msg)
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

// Queries the amm module to simulate a swap exact amount in
fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let (elys_swap_amount_in_routes,coin_in,denom_out) = process_swap_exact_amount_in_query(asset_in,&swap_operations)?;

    // Query the elys module to simulate the swap exact amount in
    let res: QuerySwapEstimationResponse = AmmQuerier::new(&deps.querier)
        .swap_estimation(
            elys_swap_amount_in_routes,
            Some(coin_in.into()),
            "0".to_string(),
        )?;
    
    let token_out_amount = match res.token_out {
        Some(token_out) => Uint128::from_str(&token_out.amount)?,
        None => return Err(ContractError::TokenOutNotFound),
    };

    // Return the asset out
    Ok(Coin {
        denom: denom_out,
        amount: token_out_amount,
    }
    .into())
}

/// Helper function to validate and process swap exact amount in query
fn process_swap_exact_amount_in_query(
    asset_in: Asset,
    swap_operations: &[SwapOperation],
) -> Result<(Vec<SwapAmountInRoute>, Coin, String), ContractError> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin in from asset in, error if asset in is not a native coin
    let coin_in = match asset_in {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_in's denom matches the first swap operation's denom_in
    if coin_in.denom != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // Get denom out from last swap operation  to be used as the return coin's denom
    let denom_out = last_op.denom_out.clone();

    // Convert the swap operations to elys swap amount in routes
    let elys_swap_amount_in_routes: Vec<SwapAmountInRoute> =
        convert_swap_operations(swap_operations.to_vec()).map_err(ContractError::ParseIntPoolID)?;

    Ok((elys_swap_amount_in_routes, coin_in,denom_out))
}

/// Helper function to validate and process swap exact amount out query
fn process_swap_exact_amount_out_query(
    asset_out: Asset,
    swap_operations: &[SwapOperation],
) -> Result<(Vec<SwapAmountOutRoute>, Coin, String), ContractError> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin out from asset out, error if asset out is not a
    // native coin because Elys does not support CW20 tokens.
    let coin_out = match asset_out {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    // Get denom in from first swap operation to be used as the return coin's denom
    let denom_in = first_op.denom_in.clone();

    // Convert the swap operations to elys swap amount out routes
    // Return an error if there was an error converting the swap
    // operations to elys swap amount out routes.
    let elys_swap_amount_out_routes: Vec<SwapAmountOutRoute> =
        convert_swap_operations(swap_operations.to_vec()).map_err(ContractError::ParseIntPoolID)?;

    Ok((elys_swap_amount_out_routes, coin_out,denom_in))
}

// Queries the elys amm module to simulate a swap exact amount out
fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let (elys_swap_amount_out_routes,coin_out,denom_in) = process_swap_exact_amount_out_query(asset_out,&swap_operations)?;

    // Query the elys amm module to simulate the swap exact amount out
    let res: QuerySwapEstimationExactAmountOutResponse = AmmQuerier::new(&deps.querier)
        .swap_estimation_exact_amount_out(
            elys_swap_amount_out_routes,
            Some(coin_out.into()),
            "0".to_string(),
        )?;
    
    let token_in_amount = match res.token_in {
        Some(token_in) => Uint128::from_str(&token_in.amount)?,
        None => return Err(ContractError::TokenInNotFound),
    };

    // Return the asset in needed
    Ok(Coin {
        denom: denom_in,
        amount: token_in_amount,
    }
    .into())
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

    let mut asset_out = Asset::new(deps.api, &ask_denom, Uint128::zero());
    let mut curr_spot_price = Decimal::zero();
    for route in &routes {
        let route_asset_out = query_simulate_swap_exact_asset_in_with_metadata(
            deps,
            route.offer_asset.clone(),
            route.operations.clone(),
            true
        )?;

        let weight = Decimal::from_ratio(route.offer_asset.amount(), asset_in.amount());
        curr_spot_price = curr_spot_price + (route_asset_out.spot_price.ok_or(ContractError::SpotPriceNotFound)? * weight);

        asset_out.add(route_asset_out.asset_out.amount())?;
    }

    let mut response = SimulateSmartSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(curr_spot_price);
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

// Queries the elys amm module to simulate a swap exact amount in with metadata
fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    let (elys_swap_amount_in_routes,coin_in,denom_out) = process_swap_exact_amount_in_query(asset_in,&swap_operations)?;

    // Query the elys module to simulate the swap exact amount in
    let res: QuerySwapEstimationResponse = AmmQuerier::new(&deps.querier)
        .swap_estimation(
            elys_swap_amount_in_routes,
            Some(coin_in.into()),
            "0".to_string(),
        )?;
    
    let token_out_amount = match res.token_out {
        Some(token_out) => Uint128::from_str(&token_out.amount)?,
        None => return Err(ContractError::TokenOutNotFound),
    };

    let mut response = SimulateSwapExactAssetInResponse {
        asset_out: Coin {
            denom: denom_out,
            amount: token_out_amount,
        }.into(),
        spot_price: None,
    };
    if include_spot_price {
        response.spot_price = Some(Decimal::from_str(&res.spot_price)?)
    }

    Ok(response)
}

// Queries the elys amm module to simulate a swap exact amount out with metadata
fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {

    let (elys_swap_amount_out_routes,coin_out,denom_in) = process_swap_exact_amount_out_query(asset_out,&swap_operations)?;

    // Query the elys amm module to simulate the swap exact amount out
    let res: QuerySwapEstimationExactAmountOutResponse = AmmQuerier::new(&deps.querier)
        .swap_estimation_exact_amount_out(
            elys_swap_amount_out_routes,
            Some(coin_out.into()),
            "0".to_string(),
        )?;
    
    let token_in_amount = match res.token_in {
        Some(token_in) => Uint128::from_str(&token_in.amount)?,
        None => return Err(ContractError::TokenInNotFound),
    };

    let mut response = SimulateSwapExactAssetOutResponse {
        asset_in: Coin {
            denom: denom_in,
            amount: token_in_amount,
        }.into(),
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(Decimal::from_str(&res.spot_price)?)
    }

    Ok(response)
}
