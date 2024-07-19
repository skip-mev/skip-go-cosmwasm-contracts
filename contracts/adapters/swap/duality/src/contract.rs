use crate::{
    error::{ContractError, ContractResult},
    state::{DEX_MODULE_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS},
};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BalanceResponse, BankQuery, Binary, Coin, CosmosMsg,
    Decimal, Deps, DepsMut, Env, Int128, MessageInfo, QueryRequest, Response, StdError, Uint128,
    WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use neutron_sdk::stargate::dex::types::{
    AllTickLiquidityRequest, AllTickLiquidityResponse, EstimateMultiHopSwapRequest,
    EstimateMultiHopSwapResponse, EstimatePlaceLimitOrderRequest, EstimatePlaceLimitOrderResponse,
    LimitOrderType, TickLiquidity::LimitOrderTranche, TickLiquidity::PoolReserves,
};
use neutron_sdk::{
    bindings::query::PageRequest,
    proto_types::neutron::dex::{MsgMultiHopSwap, MultiHopRoute},
    stargate::{
        aux::create_stargate_msg,
        dex::query::{
            get_estimate_multi_hop_swap, get_estimate_place_limit_order, get_tick_liquidity_all,
        },
    },
};

use std::str::FromStr;

use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, get_ask_denom_for_routes,
        DualityInstantiateMsg as InstantiateMsg, ExecuteMsg, MigrateMsg, QueryMsg, Route,
        SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation,
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
const MAX_SLIPPAGE_BASIS_POINTS: i64 = 2000;

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
    let swap_msg: CosmosMsg = create_duality_swap_msg(&env, coin_in, operations)?;

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
) -> ContractResult<CosmosMsg> {
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
        exit_limit_price: String::from("000000000000000000000000001"),
        pick_best_route: true,
    };

    Ok(create_stargate_msg(
        "/neutron.dex.MsgMultiHopSwap",
        swap_msg,
    ))
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
            _env,
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
            _env,
            asset_out,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSmartSwapExactAssetIn { routes, .. } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_json_binary(&query_simulate_smart_swap_exact_asset_in(
                deps, _env, ask_denom, routes,
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
                _env,
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

    // Convert the swap operations to a duality multi hop route.
    // Returns error un unsucessful conversion
    let duality_multi_hop_swap_route: MultiHopRoute =
        match get_route_from_swap_operations_for_query(swap_operations) {
            Ok(route) => route,
            Err(e) => return Err(e),
        };

    // unfortunate type conversion. should't be an issue for normal people amounts
    let amount_in: Int128 = match uint128_to_int128(coin_in.amount) {
        Ok(amount) => amount,
        Err(e) => return Err(e),
    };

    let dex_module_address: Addr = DEX_MODULE_ADDRESS.load(deps.storage)?;

    // Create the duality multi hop swap query
    let query_msg: EstimateMultiHopSwapRequest = EstimateMultiHopSwapRequest {
        // creator is the DEX for the query as it will usually have sufficient balance.
        // this balance requirement will de depricated soon.
        creator: dex_module_address.to_string(),
        // Receiver cannot be the dex, it is blocked from receiving funds
        receiver: env.contract.address.to_string(),
        routes: vec![duality_multi_hop_swap_route.hops],
        amount_in: amount_in.to_string(),
        exit_limit_price: String::from("000000000000000000000000001"),
        pick_best_route: true,
    };

    let simulation_result: EstimateMultiHopSwapResponse =
        get_estimate_multi_hop_swap(deps, query_msg)?;
    // Return the asset out
    Ok(Coin {
        denom: denom_out,
        amount: simulation_result.coin_out.amount,
    }
    .into())
}

fn query_simulate_swap_exact_asset_out(
    deps: Deps,
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

    let mut coin_in_res: Uint128 = coin_out.amount;

    // iterate over the swap operations from last to first using taker limit orders with maxAmountOut for swaps.
    for swap_operation in swap_operations.iter().rev() {
        // we use coin_in_res as the maxAmountOut on each querry. This will lead to the final iunput required to get the
        // last output after all iterations are finished
        coin_in_res = perform_duality_limit_order_query(coin_in_res, swap_operation, deps, &env)?;
    }

    // Return the asset in needed
    Ok(Coin {
        denom: denom_in,
        amount: coin_in_res,
    }
    .into())
}

// same as query_simulate_swap_exact_asset_in but also returns a spot price if required
fn query_simulate_swap_exact_asset_in_with_metadata(
    deps: Deps,
    env: Env,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    let mut response: SimulateSwapExactAssetInResponse = SimulateSwapExactAssetInResponse {
        asset_out: query_simulate_swap_exact_asset_in(
            deps,
            env,
            asset_in,
            swap_operations.clone(),
        )?,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(calculate_spot_price_multi(deps, swap_operations)?)
    }

    Ok(response)
}
// same as query_simulate_swap_exact_asset_out but also returns a spot price if required
fn query_simulate_swap_exact_asset_out_with_metadata(
    deps: Deps,
    env: Env,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetOutResponse> {
    let mut response: SimulateSwapExactAssetOutResponse = SimulateSwapExactAssetOutResponse {
        asset_in: query_simulate_swap_exact_asset_out(
            deps,
            env,
            asset_out,
            swap_operations.clone(),
        )?,
        spot_price: None,
    };

    if include_spot_price {
        response.spot_price = Some(calculate_spot_price_multi(deps, swap_operations)?)
    }

    Ok(response)
}

///////////////////
/// UNSUPPORTED ///
///////////////////
// Smart swap is not supported since it cannot happen atomically right now.
// These functions can be used as long as routes is of length 1.
// This effectively makes them the same as their non-smart counterparts:
// "query_simulate_swap_exact_asset_in" and "query_simulate_swap_exact_asset_out"
fn query_simulate_smart_swap_exact_asset_in(
    deps: Deps,
    env: Env,
    ask_denom: String,
    routes: Vec<Route>,
) -> ContractResult<Asset> {
    if routes.len() != 1 {
        return Err(ContractError::SmartSwapUnsupported);
    }
    let sim_asset_out = query_simulate_swap_exact_asset_in(
        deps,
        env,
        routes[0].offer_asset.clone(),
        routes[0].operations.clone(),
    )?;
    if *sim_asset_out.denom() == ask_denom {
        Ok(sim_asset_out)
    } else {
        Err(ContractError::SmartSwapUnexpectedOut)
    }
}
fn query_simulate_smart_swap_exact_asset_in_with_metadata(
    deps: Deps,
    env: Env,
    asset_in: Asset,
    ask_denom: String,
    routes: Vec<Route>,
    include_spot_price: bool,
) -> ContractResult<SimulateSwapExactAssetInResponse> {
    if routes.len() != 1 {
        return Err(ContractError::SmartSwapUnsupported);
    }
    let responce = query_simulate_swap_exact_asset_in_with_metadata(
        deps,
        env,
        asset_in,
        routes[0].operations.clone(),
        include_spot_price,
    )?;
    if *responce.asset_out.denom() == ask_denom {
        Ok(responce)
    } else {
        Err(ContractError::SmartSwapUnexpectedOut)
    }
}

///////////////
/// HELPERS ///
///////////////

// multi-hop-swap routes are a string array of denoms to route through
// with format [tokenA,tokenB,tokenC,tokenD]
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

// multi-hop-swap routes are a string array of denoms to route through
// with format [tokenA,tokenB,tokenC,tokenD]
pub fn get_route_from_swap_operations_for_query(
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

// Mock function to represent the Duality limit order query
fn perform_duality_limit_order_query(
    amount_out: Uint128,
    swap_operation: &SwapOperation,
    deps: Deps,
    env: &Env,
) -> Result<Uint128, ContractError> {
    // Create the bank query request for the DEX balance. We do this because simulations require a balance
    // and we don't have access to the sender's balance. The DEX should often have a sufficient balance.
    // This is a temporary workaround untill we remove balance requirements for query.
    let dex_module_address: Addr = DEX_MODULE_ADDRESS.load(deps.storage)?;

    let dex_balance_request = QueryRequest::Bank(BankQuery::Balance {
        address: dex_module_address.clone().into(),
        denom: swap_operation.denom_in.clone(),
    });

    // get the DEX balance.
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

    // convert amount_out to int.
    let max_out: Int128 = match uint128_to_int128(amount_out) {
        Ok(amount) => amount,
        Err(e) => return Err(e),
    };

    // get the tick index
    let (_, cur_tick) =
        get_spot_price_and_tick(deps, &swap_operation.denom_out, &swap_operation.denom_in)?;
    // add some safe but arbitrary slippage to satisfy some dex internals
    let tick_index_in_to_out = cur_tick + MAX_SLIPPAGE_BASIS_POINTS;
    // create the LimitOrder Message
    let query_msg = EstimatePlaceLimitOrderRequest {
        creator: dex_module_address.clone().to_string(),
        receiver: env.contract.address.to_string(),
        token_in: swap_operation.denom_in.clone(),
        token_out: swap_operation.denom_out.clone(),
        tick_index_in_to_out,
        amount_in: input_amount.to_string(),
        order_type: LimitOrderType::FillOrKill,
        // expiration_time is only valid if order_type == GOOD_TIL_TIME.
        expiration_time: None,
        max_amount_out: Some(max_out.to_string()),
    };

    // Get the result of the simulation
    let simulation_result: EstimatePlaceLimitOrderResponse =
        get_estimate_place_limit_order(deps, query_msg)?;
    // Return the input amount needed to yeild the given output amount (max_out).
    Ok(simulation_result.swap_in_coin.amount)
}

fn calculate_spot_price_multi(
    deps: Deps,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Decimal> {
    swap_operations.into_iter().try_fold(
        Decimal::one(),
        |curr_spot_price, swap_op| -> ContractResult<Decimal> {
            let (spot_price_decimal, _) =
                get_spot_price_and_tick(deps, &swap_op.denom_out, &swap_op.denom_in)?;

            // make sure to invert the price result since the expected output is the inverse of how Duality calculates price
            let division_result = Decimal::one()
                .checked_div(spot_price_decimal)
                .map_err(|e| {
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

fn new_pair_id_str(token0: &String, token1: &String) -> String {
    let mut tokens = [token0.clone(), token1.clone()];
    if token1 < token0 {
        tokens.reverse();
    }
    tokens.join("<>")
}

fn get_spot_price_and_tick(
    deps: Deps,
    token_in: &String,
    token_out: &String,
) -> ContractResult<(Decimal, i64)> {
    let query_msg = AllTickLiquidityRequest {
        pair_id: new_pair_id_str(token_in, token_out),
        token_in: token_in.to_string(),
        pagination: Some(PageRequest {
            key: Binary::from(Vec::new()),
            limit: 1,
            reverse: false,
            count_total: false,
            offset: 0,
        }),
    };

    let tick_liq_resp: AllTickLiquidityResponse = get_tick_liquidity_all(deps, query_msg)?;

    if tick_liq_resp.tick_liquidity.is_empty() {
        return Err(ContractError::NoLiquidityToParse);
    }

    let liq = &tick_liq_resp.tick_liquidity[0];

    let spot_price_str: String;
    let tick_index: i64;
    // Handle empty case
    match &liq {
        PoolReserves(reserves) => {
            spot_price_str = reserves.price_taker_to_maker.clone();
            tick_index = reserves.key.tick_index_taker_to_maker.i64();
        }
        LimitOrderTranche(tranche) => {
            spot_price_str = tranche.price_taker_to_maker.clone();
            tick_index = tranche.key.tick_index_taker_to_maker.i64();
        }
    }

    // Decimal::from fails if we supply more than 18 fractional or decimal digits.
    // Our prices can be much more persise than the 18 ddigit allowance here so a number of
    // our highest and lowest prices will not be supported on some assets. 
    let spot_price_decimal: Decimal = parse_and_validate_price(&spot_price_str)?;

    Ok((spot_price_decimal, tick_index))
}

fn parse_and_validate_price(input: &str) -> ContractResult<Decimal> {
    // find position of decinal point
    if let Some(pos) = input.find('.') {
        // get fractional and integer parts
        let fractional_part_full = &input[pos + 1..];
        let integer_part = &input[..pos];

        // only perform this logic if there are more than 18 trailling decimals points
        if fractional_part_full.len() > 18 {
            // if the 19th digit is > 5 round up.
            let round_up = fractional_part_full
                .chars()
                .nth(18)
                .unwrap()
                .to_digit(10)
                .unwrap()
                >= 5;

            // truncate the original fractiona part to 18
            let fractional_part_truncated = &fractional_part_full[..18];
            // create the full string with only 18 trailling fractional digits then create the decinal.
            let truncated_str_full = format!("{}.{}", integer_part, fractional_part_truncated);
            let mut decimal = Decimal::from_str(&truncated_str_full).map_err(|e| {
                StdError::generic_err(format!("Failed to parse truncated Decimal: {}", e))
            })?;

            // if we're rounding up we add min value
            if round_up {
                decimal += Decimal::from_str("0.000000000000000001").unwrap();
            } 

            // error if the return price is zero at this point.
            if decimal == Decimal::zero() {
                return Err(ContractError::PriceTruncateError);
            }

            // check for price deviation due to truncating.
            // we only check for deviation from original value if numerator (integer_value) is 0. Otherwise cutting least significant digits
            // will have a negligible price impact.
            let integer_value = Decimal::from_str(integer_part).map_err(|e| {
                StdError::generic_err(format!("Failed to parse truncated Decimal: {}", e))
            })?;
            if integer_value == Decimal::zero() {
                // create ints from the fractional trailing digits to compare.
                // using int128 is safe here since PercDec struct is max len 27
                let fractional_original: i128 = fractional_part_full.parse().unwrap_or(0);
                let fractional_truncated: i128 = fractional_part_truncated.parse().unwrap_or(0);
                let scale_factor = 10_i128
                    .pow((fractional_part_full.len() - fractional_part_truncated.len()) as u32);
                let fractional_truncated_scaled = fractional_truncated * scale_factor;
                let diff = (fractional_original - fractional_truncated_scaled).abs();
                let pct_diff: f64 = diff as f64 / fractional_original as f64 * 100.0;
                // error if truncating caused more than a 1% price difference
                if pct_diff > 1.0 {
                    return Err(ContractError::PriceTruncateError);
                }
                return Ok(decimal);
            }
            return Ok(decimal);
        } else {
            // price has fewer than 18 decimal places. We convert the input directly
            let spot_price = Decimal::from_str(input).map_err(|e| {
                StdError::generic_err(format!("Failed to parse spot_price as Decimal: {}", e))
            })?;
            return Ok(spot_price);
        }
    }
    // spot price has no fractional value. we convert the input directly.
    let spot_price = Decimal::from_str(input).map_err(|e| {
        StdError::generic_err(format!("Failed to parse spot_price as Decimal: {}", e))
    })?;
    Ok(spot_price)
}


///////////////
/// TESTS   ///
///////////////

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[derive(Debug, PartialEq)]
    enum ExpectedError {
        PriceTruncateError,
        GenericErr(String),
    }
    
    #[test_case("0.000000000000000010100000", "0.000000000000000010" ; "0.99% change no truncate")]
    #[test_case("2.0", "2.0" ; "single fractional point -1")]
    #[test_case("99925198949099993173.0", "99925198949099993173.0" ; "single fractional point -2")]
    #[test_case("1.1111222", "1.1111222" ; "few fractional points -1")]
    #[test_case("340251183460499231732.1111222", "340251183460499231732.1111222" ; "few fractional points -2")]
    #[test_case("123456789", "123456789" ; "no fractional points -1")]
    #[test_case("340251183460499231732", "340251183460499231732" ; "no fractional points -2")]
    #[test_case("0.111111111111111111111111111111111", "0.111111111111111111" ; "max fractional -1")]
    #[test_case("340251183460499231732.111111111111111111111111111111", "340251183460499231732.111111111111111111" ; "max fractional -2")]
    #[test_case("1.99999999999999999990000000000", "2.0" ; "rounding -1")]
    #[test_case("1.11112222333344445590000000000", "1.111122223333444456" ; "rounding -2 ")]
    #[test_case("340251183460499231732.9999999999999999999999", "340251183460499231733.0" ; "rounding -3")]
    #[test_case("99925198949099993173.11112222333344445590000000000", "99925198949099993173.111122223333444456" ; "rounding -4")]
    #[test_case("340251183460499231732.999999999999999999", "340251183460499231732.999999999999999999" ; "large decimal")]
    fn test_parse_and_validate_price(
        input: &str,
        expected: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let expected_dec = Decimal::from_str(expected).map_err(|e| {
            StdError::generic_err(format!("Failed to parse expected Decimal: {}", e))
        })?;
        let result = parse_and_validate_price(input)?;
        assert_eq!(result, expected_dec);
        Ok(())
    }

    #[test_case("0.00000000000000001011111111", ExpectedError::PriceTruncateError ; "price truncate error 1")]
    #[test_case("0.0000000000000000005", ExpectedError::PriceTruncateError ; "price truncate error 2")]
    #[test_case("0.0000000000000000099999999", ExpectedError::PriceTruncateError ; "price truncate error 3")]
    #[test_case("12345667845674567456745674567", ExpectedError::GenericErr("Value too big".to_string()) ; "too large decimal")]
    fn test_parse_and_validate_price_error(input: &str, expected_error: ExpectedError) -> Result<(), Box<dyn std::error::Error>> {
        match parse_and_validate_price(input) {
            Ok(_) => Err(Box::new(StdError::generic_err("Expected error, but got Ok"))),
            Err(e) => match expected_error {
                ExpectedError::PriceTruncateError => match e {
                    ContractError::PriceTruncateError => Ok(()),
                    _ => Err(Box::new(StdError::generic_err(format!("Unexpected error: {:?}", e)))),
                },
                ExpectedError::GenericErr(ref msg) => match e {
                    ContractError::Std(StdError::GenericErr { msg: ref err_msg, .. })if err_msg.contains(msg) => Ok(()),
                    _ => Err(Box::new(StdError::generic_err(format!("Unexpected error: {:?}", e)))),
                },
            },
        }
    }
}
