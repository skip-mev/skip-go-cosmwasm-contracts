use crate::{
    error::{ContractError, ContractResult},
    state::{
        ENTRY_POINT_CONTRACT, REGISTERED_TOKENS, SHADE_POOL_CODE_HASH, SHADE_ROUTER_CONTRACT,
        VIEWING_KEY,
    },
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, ContractInfo, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};
// use cw2::set_contract_version;
use secret_toolkit::snip20;
use skip::{
    asset::{get_current_asset_available, Asset},
    error::SkipError,
    swap::{
        get_ask_denom_for_routes, Cw20HookMsg, QueryMsg, Route,
        SimulateSmartSwapExactAssetInResponse, SimulateSwapExactAssetInResponse,
        SimulateSwapExactAssetOutResponse, SwapOperation,
    },
};

use crate::shade_swap_router_msg::{Hop, InvokeMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract: ContractInfo,
    pub shade_protocol_router_contract: ContractInfo,
    pub shade_pool_code_hash: String,
    pub viewing_key: String,
}

#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract: ContractInfo,
    pub shade_protocol_router_contract: ContractInfo,
    pub shade_pool_code_hash: String,
    pub viewing_key: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Snip20ReceiveMsg),
    TransferFundsBack { swapper: Addr, return_denom: String },
    RegisterTokens { contracts: Vec<ContractInfo> },
}

#[cw_serde]
pub struct Snip20ReceiveMsg {
    pub sender: Addr,
    pub from: Addr,
    pub amount: Uint128,
    pub memo: Option<String>,
    pub msg: Option<Binary>,
}

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    // Set contract version
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract = ContractInfo {
        address: deps
            .api
            .addr_validate(&msg.entry_point_contract.address.to_string())?,
        code_hash: msg.entry_point_contract.code_hash,
    };

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT.save(deps.storage, &checked_entry_point_contract)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract.address.to_string(),
        ))
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
    // Set contract version
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract
    let checked_entry_point_contract = ContractInfo {
        address: deps
            .api
            .addr_validate(&msg.entry_point_contract.address.to_string())?,
        code_hash: msg.entry_point_contract.code_hash,
    };

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT.save(deps.storage, &checked_entry_point_contract)?;
    SHADE_ROUTER_CONTRACT.save(deps.storage, &msg.shade_protocol_router_contract)?;
    SHADE_POOL_CODE_HASH.save(deps.storage, &msg.shade_pool_code_hash)?;
    VIEWING_KEY.save(deps.storage, &msg.viewing_key)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract.address.to_string(),
        )
        .add_attribute(
            "shade_protocol_router_contract_address",
            msg.shade_protocol_router_contract.address,
        ))
}

///////////////
/// RECEIVE ///
///////////////

// Receive is the main entry point for the contract to
// receive cw20 tokens and execute the swap
pub fn receive_snip20(
    deps: DepsMut,
    env: Env,
    mut info: MessageInfo,
    snip20_msg: Snip20ReceiveMsg,
) -> ContractResult<Response> {
    // Set the sender to the originating address that triggered the cw20 send call
    // This is later validated / enforced to be the entry point contract address
    info.sender = deps.api.addr_validate(&snip20_msg.sender.to_string())?;

    match snip20_msg.msg {
        Some(msg) => match from_binary(&msg)? {
            Cw20HookMsg::Swap { operations } => {
                execute_swap(deps, env, info, operations, snip20_msg.amount)
            }
        },
        None => Ok(Response::default()),
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
        ExecuteMsg::Receive(snip20_msg) => receive_snip20(deps, env, info, snip20_msg),
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
        ExecuteMsg::RegisterTokens { contracts } => register_tokens(deps, env, contracts),
        _ => unimplemented!(),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
    input_amount: Uint128,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract = ENTRY_POINT_CONTRACT.load(deps.storage)?;
    // Get swap router from storage
    let shade_router_contract = SHADE_ROUTER_CONTRACT.load(deps.storage)?;

    // Get pool code hash from storage
    let pool_code_hash = SHADE_POOL_CODE_HASH.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Build shade router swap message
    let mut path = vec![];
    for operation in &operations {
        path.push(Hop {
            addr: operation.pool.to_string(),
            code_hash: pool_code_hash.clone(),
        });
    }

    // Input denom will be sent to router
    let input_denom = match operations.first() {
        Some(first_op) => first_op.denom_in.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };
    // Used for transfer funds back
    let return_denom = match operations.last() {
        Some(last_op) => last_op.denom_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    // Create a response object to return
    Ok(Response::new()
        .add_attribute("action", "execute_swap")
        .add_attribute("action", "dispatch_swaps_and_transfer_back")
        // Swap router execution
        .add_message(snip20::send_msg(
            shade_router_contract.address.to_string(),
            input_amount,
            Some(to_binary(&InvokeMsg::SwapTokensForExact {
                path,
                expected_return: None,
                recipient: None,
            })?),
            None,
            None,
            255,
            shade_router_contract.code_hash,
            input_denom,
        )?)
        // TransferFundsBack message to self
        .add_message(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            code_hash: env.contract.code_hash,
            msg: to_binary(&ExecuteMsg::TransferFundsBack {
                swapper: entry_point_contract.address,
                return_denom,
            })?,
            funds: vec![],
        }))
}

fn register_tokens(
    deps: DepsMut,
    env: Env,
    contracts: Vec<ContractInfo>,
) -> ContractResult<Response> {
    let mut response = Response::new();

    for contract in contracts.iter() {
        // Add to storage for later use of code hash
        REGISTERED_TOKENS.save(deps.storage, contract.address.clone(), contract)?;
        // register receive, set viewing key, & add attribute
        response = response
            .add_attribute("register_token", contract.address.clone())
            .add_messages(vec![
                snip20::set_viewing_key_msg(
                    VIEWING_KEY.load(deps.storage)?,
                    None,
                    255,
                    contract.code_hash.clone(),
                    contract.address.to_string(),
                )?,
                snip20::register_receive_msg(
                    env.contract.code_hash.clone(),
                    None,
                    255,
                    contract.code_hash.clone(),
                    contract.address.to_string(),
                )?,
            ]);
    }

    Ok(response)
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

    // load entry point contract for sending funds to
    let entry_point_contract = match ENTRY_POINT_CONTRACT.load(deps.storage) {
        Ok(contract) => contract,
        Err(e) => return Err(SkipError::Std(e)),
    };

    // Validate return_denom
    let return_denom = match deps.api.addr_validate(&return_denom) {
        Ok(addr) => addr,
        Err(_) => return Err(SkipError::InvalidCw20Coin),
    };

    // Load token contract
    let token_contract = match REGISTERED_TOKENS.load(deps.storage, return_denom) {
        Ok(contract) => contract,
        Err(_) => return Err(SkipError::InvalidCw20Coin),
    };

    // Load viewing key for balance fetch
    let viewing_key = match VIEWING_KEY.load(deps.storage) {
        Ok(key) => key,
        Err(e) => return Err(SkipError::Std(e)),
    };

    let balance = match snip20::balance_query(
        deps.querier,
        env.contract.address.to_string(),
        viewing_key,
        255,
        token_contract.code_hash,
        token_contract.address.to_string(),
    ) {
        Ok(balance) => balance,
        Err(e) => return Err(SkipError::Std(e)),
    };

    let transfer_msg = match snip20::send_msg(
        entry_point_contract.address.to_string(),
        balance.amount,
        None,
        None,
        None,
        255,
        token_contract.code_hash,
        token_contract.address.to_string(),
    ) {
        Ok(msg) => msg,
        Err(e) => return Err(SkipError::Std(e)),
    };

    Ok(Response::new()
        .add_message(transfer_msg)
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
        } => to_binary(&query_simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => to_binary(&query_simulate_swap_exact_asset_out(
            deps,
            asset_out,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in,
            swap_operations,
            include_spot_price,
        } => to_binary(&query_simulate_swap_exact_asset_in_with_metadata(
            deps,
            asset_in,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out,
            swap_operations,
            include_spot_price,
        } => to_binary(&query_simulate_swap_exact_asset_out_with_metadata(
            deps,
            asset_out,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSmartSwapExactAssetIn { routes, .. } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_binary(&query_simulate_smart_swap_exact_asset_in(
                deps, ask_denom, routes,
            )?)
        }
        QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
            asset_in,
            routes,
            include_spot_price,
        } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_binary(&query_simulate_smart_swap_exact_asset_in_with_metadata(
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

// Queries the astroport pool contracts to simulate a swap exact amount in
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

// Queries the astroport pool contracts to simulate a multi-hop swap exact amount out
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

// Queries the astroport pool contracts to simulate a swap exact amount in with metadata
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

// Queries the astroport pool contracts to simulate a multi-hop swap exact amount out with metadata
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

// Simulates a swap exact amount in request, returning the asset out and optionally the reverse simulation responses
fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<SimulationResponse>)> {
    let (asset_out, responses) = swap_operations.iter().try_fold(
        (asset_in, Vec::new()),
        |(asset_out, mut responses), operation| -> Result<_, ContractError> {
            // Get the astroport offer asset type
            let astroport_offer_asset = asset_out.into_astroport_asset(deps.api)?;

            // Query the astroport pool contract to get the simulation response
            let res: SimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::Simulation {
                    offer_asset: astroport_offer_asset,
                    ask_asset_info: None,
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

// Simulates a swap exact amount out request, returning the asset in needed and optionally the reverse simulation responses
fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
    include_responses: bool,
) -> ContractResult<(Asset, Vec<ReverseSimulationResponse>)> {
    let (asset_in, responses) = swap_operations.iter().rev().try_fold(
        (asset_out, Vec::new()),
        |(asset_in_needed, mut responses), operation| -> Result<_, ContractError> {
            // Get the astroport ask asset type
            let astroport_ask_asset = asset_in_needed.into_astroport_asset(deps.api)?;

            // Query the astroport pool contract to get the reverse simulation response
            let res: ReverseSimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::ReverseSimulation {
                    offer_asset_info: None,
                    ask_asset: astroport_ask_asset,
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

// Calculate the spot price using simulation responses
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

// Calculates the spot price using reverse simulaation responses
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
