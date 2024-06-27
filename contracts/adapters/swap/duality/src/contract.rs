use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, Int128, MessageInfo,
    QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use neutron_sdk::bindings::dex::query::{
    DexQuery::EstimateMultiHopSwap, EstimateMultiHopSwapResponse,
};
use neutron_sdk::{
    bindings::dex::types::{MultiHopRoute, PrecDec},
    proto_types::neutron::dex::MsgMultiHopSwap,
};

use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, get_ask_denom_for_routes, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryMsg, Route, SwapOperation,
    },
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

    //build duality Swap message
    let swap_msg = create_duality_swap_msg(&env, coin_in, operations)?;

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

// Creates the duality swap message
fn create_duality_swap_msg(
    env: &Env,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<CosmosMsg<Mul>> {
    // Convert the swap operations into a Duality multi hop swap route.
    let route = match get_route_from_swap_operations(swap_operations) {
        Ok(route) => route,
        Err(e) => return Err(e),
    };

    // Create the duality multi hop swap message
    let swap_msg = MsgMultiHopSwap {
        creator: env.contract.address.to_string(),
        receiver: env.contract.address.to_string(),
        routes: vec![route],
        amount_in: coin_in.amount.into(),
        exit_limit_price: PrecDec {
            i: "0.00000001".to_string(),
        }
        .into(),
        pick_best_route: true,
    }.into();

    let cosmos_msg = match convert_to_cosmos_msg(swap_msg) {
        Ok(response) => response,
        Err(err) => return Err(ContractError::from(err)),
    };

    Ok(cosmos_msg)
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
            _env,
            asset_in,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => to_json_binary(&query_simulate_swap_exact_asset_out(
            deps,
            _env,
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
    env: Env,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Get coin in from asset in, error if asset in is not a
    // native coin because Duality does not support CW20 tokens.
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

    // Convert the swap operations to a duality multi hop route .
    // Returns error un unsucessful conversion
    let duality_multi_hop_swap_route: MultiHopRoute =
        match get_route_from_swap_operations(swap_operations) {
            Ok(route) => route,
            Err(e) => return Err(e),
        };

    let amount_in: Int128 = match uint128_to_int128(coin_in.amount) {
        Ok(route) => route,
        Err(e) => return Err(e),
    };

    // Create the duality multi hop swap message
    let query_msg = EstimateMultiHopSwap {
        creator: env.contract.address.to_string(),
        receiver: env.contract.address.to_string(),
        routes: vec![duality_multi_hop_swap_route],
        amount_in: amount_in,
        exit_limit_price: PrecDec {
            i: "0.00000001".to_string(),
        },
        pick_best_route: true,
    };

    // Serialize the query message
    let binary_msg = to_json_binary(&query_msg)?;

    let simulation_result: StdResult<EstimateMultiHopSwapResponse> = deps
        .querier
        .query(&QueryRequest::Stargate {
            path: "/neutron.dex.Query/EstimateMultiHopSwap".to_string(),
            data: binary_msg,
        })
        .map_err(|e| StdError::generic_err(format!("Simulation failed")));

    // Extract the coin_out field from the response
    let out_coin = match simulation_result {
        Ok(response) => response.coin_out,
        Err(err) => return Err(ContractError::from(err)),
    };

    // Return the asset out
    Ok(Coin {
        denom: denom_out,
        amount: out_coin.amount,
    }
    .into())
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
    env: Env,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    unimplemented!()
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<EstimateMultiHopSwapResponse> {
    unimplemented!()
}

fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<EstimateMultiHopSwapResponse> {
    unimplemented!()
}

// Queries the osmosis poolmanager module to simulate a swap exact amount out with metadata
fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<EstimateMultiHopSwapResponse> {
    unimplemented!()
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    unimplemented!()
}

///////////////
/// HELPERS ///
///////////////

// multi-hop-swap routes are a string array of denoms to route through
// with formal [tokenA,tokenB,tokenC,tokenD]
pub fn get_route_from_swap_operations(
    swap_operations: Vec<SwapOperation>,
) -> Result<neutron_sdk::proto_types::neutron::dex::MultiHopRoute, ContractError> {
    if swap_operations.is_empty() {
        return Err(ContractError::SwapOperationsEmpty);
    }

    let mut route = vec![
        swap_operations[0].denom_in.clone(),
        swap_operations[0].denom_out.clone(),
    ];
    let mut last_denom_out = &swap_operations[0].denom_out;

    for operation in swap_operations.iter().skip(1) {
        if &operation.denom_in != last_denom_out {
            return Err(ContractError::SwapOperationDenomMismatch);
        }
        route.push(operation.denom_out.clone());
        last_denom_out = &operation.denom_out;
    }

    Ok(neutron_sdk::proto_types::neutron::dex::MultiHopRoute { hops: route })
}

fn uint128_to_int128(u: Uint128) -> Result<Int128, ContractError> {
    let value = u.u128();
    if value > i128::MAX as u128 {
        return Err(ContractError::ConversionError);
    }
    Ok(Int128::from(value as i128))
}

// Implement the conversion
pub fn convert_to_cosmos_msg(
    swap_msg: MsgMultiHopSwap,
) -> Result<CosmosMsg, ContractError> {
    let msg = CosmosMsg::Custom(swap_msg);

    Ok(msg)
}
