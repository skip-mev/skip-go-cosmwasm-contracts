use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    EstimateSwapExactAmountInResponse, EstimateSwapExactAmountOutResponse, MsgSwapExactAmountIn,
    PoolmanagerQuerier, SpotPriceResponse, SwapAmountInRoute, SwapAmountOutRoute,
};
use skip::{
    asset::Asset,
    proto_coin::ProtoCoin,
    swap::{
        convert_swap_operations, execute_transfer_funds_back, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryMsg, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse,
        SwapOperation,
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

    let return_denom = match operations.last() {
        Some(last_op) => last_op.denom_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // Create the osmosis poolmanager swap exact amount in message
    let swap_msg = create_osmosis_swap_msg(&env, coin_in, operations)?;

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

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Creates the osmosis poolmanager swap exact amount in message
fn create_osmosis_swap_msg(
    env: &Env,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<CosmosMsg> {
    // Convert the swap operations to osmosis swap amount in routes
    // Return an error if there was an error converting the swap
    // operations to osmosis swap amount in routes.
    let osmosis_swap_amount_in_routes: Vec<SwapAmountInRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;

    // Create the osmosis poolmanager swap exact amount in message
    // The token out min amount is set to 1 because we are not concerned
    // with the minimum amount in this contract, that gets verified in the
    // entry point contract.
    let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
        sender: env.contract.address.to_string(),
        routes: osmosis_swap_amount_in_routes,
        token_in: Some(ProtoCoin(coin_in).into()),
        token_out_min_amount: "1".to_string(),
    }
    .into();

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
    }
    .map_err(From::from)
}

// Queries the osmosis poolmanager module to simulate a swap exact amount in
fn query_simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin in from asset in, error if asset in is not a
    // native coin because Osmosis does not support CW20 tokens.
    let coin_in = match asset_in {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_in's denom is the same as the first swap operation's denom in
    if coin_in.denom != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // Get denom out from last swap operation  to be used as the return coin's denom
    let denom_out = last_op.denom_out.clone();

    // Convert the swap operations to osmosis swap amount in routes
    // Return an error if there was an error converting the swap
    // operations to osmosis swap amount in routes.
    let osmosis_swap_amount_in_routes: Vec<SwapAmountInRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;

    // Query the osmosis poolmanager module to simulate the swap exact amount in
    let res: EstimateSwapExactAmountInResponse = PoolmanagerQuerier::new(&deps.querier)
        .estimate_swap_exact_amount_in(
            osmosis_swap_amount_in_routes.first().unwrap().pool_id,
            coin_in.to_string(),
            osmosis_swap_amount_in_routes,
        )?;

    // Return the asset out
    Ok(Coin {
        denom: denom_out,
        amount: Uint128::from_str(&res.token_out_amount)?,
    }
    .into())
}

// Queries the osmosis poolmanager module to simulate a swap exact amount out
fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
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

    // Get denom in from first swap operation to be used as the return coin's denom
    let denom_in = first_op.denom_in.clone();

    // Convert the swap operations to osmosis swap amount out routes
    // Return an error if there was an error converting the swap
    // operations to osmosis swap amount out routes.
    let osmosis_swap_amount_out_routes: Vec<SwapAmountOutRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;

    // Query the osmosis poolmanager module to simulate the swap exact amount out
    let res: EstimateSwapExactAmountOutResponse = PoolmanagerQuerier::new(&deps.querier)
        .estimate_swap_exact_amount_out(
            osmosis_swap_amount_out_routes.first().unwrap().pool_id,
            osmosis_swap_amount_out_routes,
            coin_out.to_string(),
        )?;

    // Return the asset in needed
    Ok(Coin {
        denom: denom_in,
        amount: Uint128::from_str(&res.token_in_amount)?,
    }
    .into())
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
        response.spot_price = Some(calculate_spot_price(
            &PoolmanagerQuerier::new(&deps.querier),
            swap_operations,
        )?)
    }

    Ok(response)
}

// Queries the osmosis poolmanager module to simulate a swap exact amount out with metadata
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
        response.spot_price = Some(calculate_spot_price(
            &PoolmanagerQuerier::new(&deps.querier),
            swap_operations,
        )?)
    }

    Ok(response)
}

// Calculates the spot price for a given vector of swap operations
fn calculate_spot_price(
    querier: &PoolmanagerQuerier<Empty>,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Decimal> {
    let spot_price = swap_operations.into_iter().try_fold(
        Decimal::one(),
        |curr_spot_price, swap_op| -> ContractResult<Decimal> {
            let spot_price_res: SpotPriceResponse = querier.spot_price(
                swap_op.pool.parse()?, // should not error since we already parsed it earlier
                swap_op.denom_in,
                swap_op.denom_out,
            )?;
            Ok(curr_spot_price.checked_mul(Decimal::from_str(&spot_price_res.spot_price)?)?)
        },
    )?;

    Ok(spot_price)
}
