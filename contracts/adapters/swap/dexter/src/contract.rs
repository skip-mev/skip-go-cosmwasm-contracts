use std::str::FromStr;

use crate::{
    error::{ContractError, ContractResult},
    state::{DEXTER_ROUTER_ADDRESS, DEXTER_VAULT_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS},
};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw_utils::one_coin;
use dexter::{
    pool::{self, ResponseType, SpotPrice},
    router::{ExecuteMsg as RouterExecuteMsg, HopSwapRequest, QueryMsg as RouterQueryMsg},
    vault::{PoolInfoResponse, QueryMsg as VaultQueryMsg},
};
use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, Cw20HookMsg, DexterAdapterInstantiateMsg, ExecuteMsg,
        MigrateMsg, QueryMsg, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse,
        SwapOperation,
    },
};

// const DEXTER_VAULT_ADDRESS: &str = "persistence1k8re7jwz6rnnwrktnejdwkwnncte7ek7gt29gvnl3sdrg9mtnqkstujtpg";
// const DEXTER_ROUTER_ADDRESS: &str = "persistence132xmxm33vwjlur2pszl4hu9r32lqmqagvunnuc5hq4htps7rr3kqsf4dsk";

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
    msg: DexterAdapterInstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    let dexter_vault_contract_address =
        deps.api.addr_validate(&msg.dexter_vault_contract_address)?;
    let dexter_router_contract_address = deps
        .api
        .addr_validate(&msg.dexter_router_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;
    DEXTER_ROUTER_ADDRESS.save(deps.storage, &dexter_router_contract_address)?;
    DEXTER_VAULT_ADDRESS.save(deps.storage, &dexter_vault_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
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
            // validate that there's at least one swap operation
            if operations.is_empty() {
                return Err(ContractError::SwapOperationsEmpty);
            }

            let coin = one_coin(&info)?;

            // validate that the one coin is the same as the first swap operation's denom in
            if coin.denom != operations.first().unwrap().denom_in {
                return Err(ContractError::CoinInDenomMismatch);
            }

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
            panic!("NOT IMPLEMENTED");
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
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;
    let dexter_router_contract_address = DEXTER_ROUTER_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Create a response object to return
    let response: Response = Response::new().add_attribute("action", "execute_swap");

    let mut hop_swap_requests = vec![];

    for operation in &operations {
        let pool_id: u64 = operation
            .pool
            .parse()
            .map_err(|_| ContractError::PoolIdParseError)?;
        let pool_id_u128 = Uint128::from(pool_id);

        hop_swap_requests.push(HopSwapRequest {
            pool_id: pool_id_u128,
            asset_in: dexter::asset::AssetInfo::native_token(operation.denom_in.clone()),
            asset_out: dexter::asset::AssetInfo::native_token(operation.denom_out.clone()),
        });
    }

    let dexter_router_msg = RouterExecuteMsg::ExecuteMultihopSwap {
        requests: hop_swap_requests,
        recipient: None,
        offer_amount: amount_in,
        // doing this since we would validate it anyway in the entrypoint contract from where swap adapter is called
        minimum_receive: None,
    };

    let denom_in = operations.first().unwrap().denom_in.clone();

    let dexter_router_wasm_msg = WasmMsg::Execute {
        contract_addr: dexter_router_contract_address.to_string(),
        msg: to_json_binary(&dexter_router_msg)?,
        funds: vec![Coin {
            denom: denom_in,
            amount: amount_in,
        }],
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

    Ok(response
        .add_message(dexter_router_wasm_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swaps_and_transfer_back"))
}

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

// Queries the dexter pool contracts to simulate a swap exact amount in
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

    let asset_out = simulate_swap_exact_asset_in(deps, asset_in, swap_operations)?;

    // Return the asset out
    Ok(asset_out)
}

// Queries the dexter pool contracts to simulate a multi-hop swap exact amount out
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

    let asset_in = simulate_swap_exact_asset_out(deps, asset_out, swap_operations)?;

    // Return the asset in needed
    Ok(asset_in)
}

// Queries the dexter pool contracts to simulate a swap exact amount in with metadata
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

    // Simulate the swap exact amount in
    let asset_out = simulate_swap_exact_asset_in(deps, asset_in.clone(), swap_operations.clone())?;

    // Create the response
    let mut response = SimulateSwapExactAssetInResponse {
        asset_out,
        spot_price: None,
    };

    // Include the spot price in the response if requested
    if include_spot_price {
        let spot_price = calculate_spot_price(deps, swap_operations)?;
        response.spot_price = Some(spot_price);
    }

    Ok(response)
}

// Queries the dexter pool contracts to simulate a multi-hop swap exact amount out with metadata
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

    // Simulate the swap exact amount out
    let asset_in = simulate_swap_exact_asset_out(deps, asset_out.clone(), swap_operations.clone())?;

    // Create the response
    let mut response = SimulateSwapExactAssetOutResponse {
        asset_in,
        spot_price: None,
    };

    // Include the spot price in the response if requested
    if include_spot_price {
        let spot_price = calculate_spot_price(deps, swap_operations)?;
        response.spot_price = Some(spot_price);
    }

    Ok(response)
}

// Simulates a swap exact amount in request, returning the asset out and optionally the reverse simulation responses
fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let dexter_router_address = DEXTER_ROUTER_ADDRESS.load(deps.storage)?;

    let mut hop_swap_requests: Vec<HopSwapRequest> = vec![];
    for operation in &swap_operations {
        let pool_id: u64 = operation.pool.parse().unwrap();
        let pool_id_u128 = Uint128::from(pool_id);

        hop_swap_requests.push(HopSwapRequest {
            pool_id: pool_id_u128,
            asset_in: dexter::asset::AssetInfo::native_token(operation.denom_in.clone()),
            asset_out: dexter::asset::AssetInfo::native_token(operation.denom_out.clone()),
        });
    }

    let dexter_router_query = RouterQueryMsg::SimulateMultihopSwap {
        multiswap_request: hop_swap_requests,
        swap_type: dexter::vault::SwapType::GiveIn {},
        amount: asset_in.amount(),
    };

    let dexter_router_response: dexter::router::SimulateMultiHopResponse = deps
        .querier
        .query_wasm_smart(dexter_router_address, &dexter_router_query)?;

    if let ResponseType::Success {} = dexter_router_response.response {
        // Get the asset out
        let last_response = dexter_router_response.swap_operations.last().unwrap();

        let asset_out = Asset::Native(Coin {
            denom: last_response.asset_out.to_string(),
            amount: last_response.received_amount,
        });

        // Return the asset out and optionally the simulation responses
        Ok(asset_out)
    } else {
        Err(ContractError::SimulationError)
    }
}

// Simulates a swap exact amount out request, returning the asset in needed and optionally the reverse simulation responses
fn simulate_swap_exact_asset_out(
    deps: Deps,
    asset_out: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    let dexter_router_address = DEXTER_ROUTER_ADDRESS.load(deps.storage)?;

    let mut hop_swap_requests: Vec<HopSwapRequest> = vec![];
    for operation in &swap_operations {
        let pool_id: u64 = operation.pool.parse().unwrap();
        let pool_id_u128 = Uint128::from(pool_id);

        hop_swap_requests.push(HopSwapRequest {
            pool_id: pool_id_u128,
            asset_in: dexter::asset::AssetInfo::native_token(operation.denom_in.clone()),
            asset_out: dexter::asset::AssetInfo::native_token(operation.denom_out.clone()),
        });
    }

    let dexter_router_query = RouterQueryMsg::SimulateMultihopSwap {
        multiswap_request: hop_swap_requests,
        swap_type: dexter::vault::SwapType::GiveOut {},
        amount: asset_out.amount(),
    };

    let dexter_router_response: dexter::router::SimulateMultiHopResponse = deps
        .querier
        .query_wasm_smart(dexter_router_address, &dexter_router_query)?;

    if let ResponseType::Success {} = dexter_router_response.response {
        // Get the asset out
        let first_response = dexter_router_response.swap_operations.first().unwrap();

        let asset_in = Asset::Native(Coin {
            denom: first_response.asset_in.to_string(),
            amount: first_response.offered_amount,
        });

        // Return the asset out and optionally the simulation responses
        Ok(asset_in)
    } else {
        Err(ContractError::SimulationError)
    }
}

// find spot prices for all the pools in the swap operations
fn calculate_spot_price(
    deps: Deps,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Decimal> {
    let dexter_vault_address = DEXTER_VAULT_ADDRESS.load(deps.storage)?;
    let mut final_price = Decimal::one();
    for operation in &swap_operations {
        let pool_id: u64 = operation.pool.parse().unwrap();
        let pool_id_u128 = Uint128::from(pool_id);

        let pool_info: PoolInfoResponse = deps.querier.query_wasm_smart(
            dexter_vault_address.clone(),
            &VaultQueryMsg::GetPoolById {
                pool_id: pool_id_u128,
            },
        )?;

        let spot_price: SpotPrice = deps.querier.query_wasm_smart(
            pool_info.pool_addr,
            &pool::QueryMsg::SpotPrice {
                offer_asset: dexter::asset::AssetInfo::native_token(operation.denom_in.clone()),
                ask_asset: dexter::asset::AssetInfo::native_token(operation.denom_out.clone()),
            },
        )?;

        final_price = final_price
            .checked_mul(Decimal::from_str(&spot_price.price_including_fee.to_string()).unwrap())
            .unwrap();
    }

    Ok(final_price)
}
