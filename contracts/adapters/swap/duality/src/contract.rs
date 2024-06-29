use crate::{
    error::{ContractError, ContractResult},
    state::{DEX_MODULE_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS},
};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BalanceResponse, BankQuery, Binary, Coin, CosmosMsg,
    Decimal, Deps, DepsMut, Empty, Env, Int128, MessageInfo, QueryRequest, Response, StdError,
    StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use neutron_sdk::bindings::dex::msg::DexMsg;
use neutron_sdk::bindings::dex::msg::DexMsg::MultiHopSwap;
use neutron_sdk::bindings::dex::query::DexQuery;
use neutron_sdk::bindings::dex::types::{LimitOrderType, Liquidity, MultiHopRoute, PrecDec};
use neutron_sdk::bindings::query::PageRequest;
use neutron_sdk::bindings::{
    dex::query::{
        AllTickLiquidityResponse,
        DexQuery::{EstimateMultiHopSwap, EstimatePlaceLimitOrder, TickLiquidityAll},
        EstimateMultiHopSwapResponse, EstimatePlaceLimitOrderResponse,
    },
    query::NeutronQuery,
};
use std::str::FromStr;

use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, get_ask_denom_for_routes,
        DualityInstatiateMsg as InstantiateMsg, ExecuteMsg, MigrateMsg, QueryMsg, Route,
        SwapOperation,
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

    // Validate dex module address
    let checked_dex_module_address = deps.api.addr_validate(&msg.dex_module_address)?;

    // Store the module address
    DEX_MODULE_ADDRESS.save(deps.storage, &checked_dex_module_address)?;

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
    let swap_msg: CosmosMsg<DexMsg> = create_duality_swap_msg(&env, coin_in, operations)?;

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
) -> ContractResult<CosmosMsg<DexMsg>> {
    // Convert the swap operations into a Duality multi hop swap route.
    let route = match get_route_from_swap_operations(swap_operations) {
        Ok(route) => route,
        Err(e) => return Err(e),
    };

    // Create the duality multi hop swap message
    let swap_msg = MultiHopSwap {
        receiver: env.contract.address.to_string(),
        routes: vec![route],
        amount_in: coin_in.amount.into(),
        exit_limit_price: PrecDec {
            i: "0.00000001".to_string(),
        },
        pick_best_route: true,
    };

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
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
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
    deps: Deps<NeutronQuery>,
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
        Ok(amount) => amount,
        Err(e) => return Err(e),
    };

    // Create the duality multi hop swap message
    let query_msg: neutron_sdk::bindings::dex::query::DexQuery = EstimateMultiHopSwap {
        creator: env.contract.address.to_string(),
        receiver: env.contract.address.to_string(),
        routes: vec![duality_multi_hop_swap_route],
        amount_in: amount_in,
        exit_limit_price: PrecDec {
            i: "0.00000001".to_string(),
        },
        pick_best_route: true,
    };

    let simulation_result: EstimateMultiHopSwapResponse = deps.querier.query(&query_msg.into())?;

    // Return the asset out
    Ok(Coin {
        denom: denom_out,
        amount: simulation_result.coin_out.amount,
    }
    .into())
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps<NeutronQuery>,
    env: Env,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };
    // Get coin out from asset out, error if asset in is not a
    // native coin because Duality does not support CW20 tokens.
    let coin_out = match asset_out {
        Asset::Native(coin) => coin,
        _ => return Err(ContractError::AssetNotNative),
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }
    let denom_in: String = first_op.denom_in.clone();

    let mut coin_in_res = coin_out.amount.clone();

    // iterate over the swap operations from last to first using taker limit orders with maxAmountOut for swaps.
    for swap_operation in swap_operations.iter().rev() {
        // we use coin_in_res as the maxAmountOut on each querry. This will lead to the dinal iunput required to get the
        // last output after all iterations are finished
        coin_in_res = perform_duality_limit_order_query(coin_in_res, swap_operation, deps, &env)?;
    }

    // Return the asset in needed
    Ok(Coin {
        denom: denom_in,
        amount: coin_in_res,
    }
    .into())
    // convert the swap route to a
}

fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps<NeutronQuery>,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<EstimateMultiHopSwapResponse> {
    unimplemented!()
}

fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps<NeutronQuery>,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<EstimateMultiHopSwapResponse> {
    unimplemented!()
}

// Queries the osmosis poolmanager module to simulate a swap exact amount out with metadata
fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps<NeutronQuery>,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<EstimateMultiHopSwapResponse> {
    unimplemented!()
}

fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps<NeutronQuery>,
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
) -> Result<MultiHopRoute, ContractError> {
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

    Ok(MultiHopRoute { hops: route })
}

fn uint128_to_int128(u: Uint128) -> Result<Int128, ContractError> {
    let value = u.u128();
    if value > i128::MAX as u128 {
        return Err(ContractError::ConversionError);
    }
    Ok(Int128::from(value as i128))
}
fn int128_to_uint128(i: Int128) -> Result<Uint128, ContractError> {
    let value = i.i128();
    if value < 0 {
        return Err(ContractError::ConversionError);
    }
    Ok(Uint128::from(value as u128))
}
// Implement the conversion
pub fn convert_to_cosmos_msg(swap_msg: DexMsg) -> Result<CosmosMsg<Empty>, ContractError> {
    let msg = CosmosMsg::Custom(swap_msg.into());

    Ok(msg)
}

// Mock function to represent the Duality limit order query
fn perform_duality_limit_order_query(
    amount_out: Uint128,
    swap_operation: &SwapOperation,
    deps: Deps<NeutronQuery>,
    env: &Env,
) -> Result<Uint128, ContractError> {
    let max_tick: i64 = 559680;
    let min_tick: i64 = -559680;

    // Since we are placing a fill-or-kill maker limit order and we are not price sensitive
    // we either use mintick or maxtick to place the limit order depending on swap direction.
    // this will ensure that liimit order sees all possible liquidity as valid while still
    // iterating over the best prices first using Duality's autoswap feature.
    let index;
    if swap_operation.denom_in > swap_operation.denom_out {
        index = min_tick
    } else if swap_operation.denom_in < swap_operation.denom_out {
        index = max_tick
    } else {
        return Err(ContractError::SameSwapDenoms);
    }

    // Create the bank query request for the DEX balance. We do this because simulations require
    let dex_module_address: Addr = DEX_MODULE_ADDRESS.load(deps.storage)?;

    let dex_balance_request = QueryRequest::Bank(BankQuery::Balance {
        address: dex_module_address.clone().into(),
        denom: swap_operation.denom_in.clone(),
    });

    // get the DEX balance. Use the DEX as the caller in the Limit Order
    // simulation since it will usually have the required balance
    let dex_balance_simulation_result: BalanceResponse =
        match deps.querier.query(&dex_balance_request) {
            Ok(result) => result,
            Err(err) => return Err(ContractError::from(err)),
        };

    // set dex balance to be the input amount
    let input_amount: Int128 = match uint128_to_int128(dex_balance_simulation_result.amount.amount)
    {
        Ok(amount) => amount,
        Err(e) => return Err(e),
    };

    // set dex balance to be the input amount
    let max_out: Int128 = match uint128_to_int128(amount_out) {
        Ok(amount) => amount,
        Err(e) => return Err(e),
    };

    // create the LimitOrder Message
    let query_msg = EstimatePlaceLimitOrder {
        creator: dex_module_address.clone().to_string(),
        receiver: env.contract.address.to_string(),
        token_in: swap_operation.denom_in.clone(),
        token_out: swap_operation.denom_out.clone(),
        tick_index_in_to_out: index,
        amount_in: input_amount,
        order_type: LimitOrderType::FillOrKill,
        // expiration_time is only valid if order_type == GOOD_TIL_TIME.
        expiration_time: None,
        max_amount_out: Some(max_out),
    };

    // Get the result of the simulation
    let simulation_result: EstimatePlaceLimitOrderResponse =
        match deps.querier.query(&query_msg.into()) {
            Ok(result) => result,
            Err(err) => return Err(ContractError::from(err)),
        };

    // Return the amount in needed for the given amount out.
    Ok(simulation_result.swap_in_coin.amount)
}

fn calculate_spot_price(
    deps: Deps<NeutronQuery>,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Decimal> {
    swap_operations.into_iter().try_fold(
        Decimal::one(),
        |curr_spot_price, swap_op| -> ContractResult<Decimal> {
            let msg = DexQuery::TickLiquidityAll {
                pair_id: new_pair_id_str(swap_op.denom_out.clone(), swap_op.denom_out),
                token_in: swap_op.denom_in,
                pagination: Some(PageRequest {
                    key: Binary::from(Vec::new()),
                    limit: 1,
                    reverse: false,
                    count_total: false,
                    offset: 0,
                }),
            };
            let tick_liq_resp: AllTickLiquidityResponse = deps.querier.query(&msg.into())?;

            let liq = &tick_liq_resp.tick_liquidity[0];

            // Handle empty case
            let spot_price: String;
            match &liq.liquidity {
                Liquidity::PoolReserves(reserves) => {
                    spot_price = reserves.price_taker_to_maker.i.clone();
                }
                Liquidity::LimitOrderTranche(tranche) => {
                    spot_price = tranche.price_taker_to_maker.i.clone();
                }
            }
    
            // Convert spot_price to Decimal using its string representation
            let spot_price_decimal = Decimal::from_str(&spot_price).map_err(|e| {
                StdError::generic_err(format!("Failed to parse spot_price as Decimal: {}", e))
            })?;

            let division_result = Decimal::one().checked_div(spot_price_decimal).map_err(|e| {
                StdError::generic_err(format!("Failed to perform price division: {}", e))
            })?;

            // Perform the checked multiplication
            let result = curr_spot_price.checked_mul(division_result).map_err(|e| {
                StdError::generic_err(format!("Failed to perform price multiplication: {}", e))
            })?;

            // Return the result
            Ok(result)
        },
    )
}

fn new_pair_id_str(token0: String, token1: String) -> String {
    let tokens = vec![token0.clone(), token1.clone()];
    if token1 < token0 {
        tokens.iter().rev();
    }
    return tokens.join("<>");
}
