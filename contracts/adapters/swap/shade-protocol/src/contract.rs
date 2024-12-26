use crate::{
    error::{ContractError, ContractResult},
    state::{
        ENTRY_POINT_CONTRACT, REGISTERED_TOKENS, SHADE_POOL_CODE_HASH, SHADE_ROUTER_CONTRACT,
        VIEWING_KEY,
    },
};
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, ContractInfo, Deps, DepsMut, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use secret_skip::{asset::Asset, snip20::Snip20ReceiveMsg, swap::SwapOperation};
// use cw2::set_contract_version;
use cw20::Cw20Coin;
use secret_toolkit::snip20;

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, Snip20HookMsg},
    shade_swap_router_msg as shade_router,
};

// Contract name and version used for migration.
/*
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

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

    ENTRY_POINT_CONTRACT.save(deps.storage, &checked_entry_point_contract)?;

    let checked_shade_router_contract = ContractInfo {
        address: deps
            .api
            .addr_validate(&msg.shade_router_contract.address.to_string())?,
        code_hash: msg.shade_router_contract.code_hash,
    };

    SHADE_ROUTER_CONTRACT.save(deps.storage, &checked_shade_router_contract)?;

    VIEWING_KEY.save(deps.storage, &msg.viewing_key)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract.address.to_string(),
        )
        .add_attribute(
            "shade_router_contract_address",
            checked_shade_router_contract.address.to_string(),
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

    ENTRY_POINT_CONTRACT.save(deps.storage, &checked_entry_point_contract)?;

    let checked_shade_router_contract = ContractInfo {
        address: deps
            .api
            .addr_validate(&msg.shade_router_contract.address.to_string())?,
        code_hash: msg.shade_router_contract.code_hash,
    };

    SHADE_ROUTER_CONTRACT.save(deps.storage, &checked_shade_router_contract)?;

    VIEWING_KEY.save(deps.storage, &msg.viewing_key)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract.address.to_string(),
        )
        .add_attribute(
            "shade_router_contract_address",
            msg.shade_router_contract.address,
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
            Snip20HookMsg::Swap { operations } => {
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
        // Tokens must be registered before they can be swapped
        ExecuteMsg::RegisterTokens { contracts } => register_tokens(deps, env, contracts),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
    input_amount: Uint128,
) -> ContractResult<Response> {
    // Get entry point contract from storage
    let entry_point_contract = ENTRY_POINT_CONTRACT.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Get pool code hash from storage
    let shade_pool_code_hash = SHADE_POOL_CODE_HASH.load(deps.storage)?;

    // Build shade router swap message
    let mut path = vec![];
    for operation in &operations {
        path.push(shade_router::Hop {
            addr: operation.pool.to_string(),
            code_hash: shade_pool_code_hash.clone(),
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

    let input_denom_contract = REGISTERED_TOKENS.load(
        deps.storage,
        deps.api.addr_validate(&input_denom.to_string())?,
    )?;

    // Get shade router contract from storage
    let shade_router_contract = SHADE_ROUTER_CONTRACT.load(deps.storage)?;

    // Create a response object to return
    Ok(Response::new()
        .add_attribute("action", "execute_swap")
        .add_attribute("action", "dispatch_swaps_and_transfer_back")
        // Swap router execution
        .add_message(snip20::send_msg_with_code_hash(
            shade_router_contract.address.to_string(),
            Some(shade_router_contract.code_hash),
            input_amount,
            Some(to_binary(&shade_router::InvokeMsg::SwapTokensForExact {
                path,
                expected_return: None,
                recipient: None,
            })?),
            None,
            None,
            0,
            input_denom_contract.code_hash,
            input_denom_contract.address.to_string(),
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

    let viewing_key = VIEWING_KEY.load(deps.storage)?;

    for contract in contracts.iter() {
        // Add to storage for later use of code hash
        REGISTERED_TOKENS.save(deps.storage, contract.address.clone(), contract)?;
        // register receive, set viewing key, & add attribute
        response = response
            .add_attribute("register_token", contract.address.clone())
            .add_messages(vec![
                snip20::set_viewing_key_msg(
                    viewing_key.clone(),
                    None,
                    0,
                    contract.code_hash.clone(),
                    contract.address.to_string(),
                )?,
                snip20::register_receive_msg(
                    env.contract.code_hash.clone(),
                    None,
                    0,
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
    _swapper: Addr,
    return_denom: String,
) -> ContractResult<Response> {
    // Ensure the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Validate return_denom
    let return_denom = match deps.api.addr_validate(&return_denom) {
        Ok(addr) => addr,
        Err(_) => return Err(ContractError::InvalidSnip20Coin),
    };

    // Load token contract
    let token_contract = match REGISTERED_TOKENS.load(deps.storage, return_denom) {
        Ok(contract) => contract,
        Err(_) => return Err(ContractError::InvalidSnip20Coin),
    };

    let viewing_key = VIEWING_KEY.load(deps.storage)?;

    let balance = match snip20::balance_query(
        deps.querier,
        env.contract.address.to_string(),
        viewing_key,
        0,
        token_contract.code_hash.clone(),
        token_contract.address.to_string(),
    ) {
        Ok(balance) => balance,
        Err(e) => return Err(ContractError::Std(e)),
    };

    let entry_point_contract = ENTRY_POINT_CONTRACT.load(deps.storage)?;

    let send_msg = match snip20::send_msg_with_code_hash(
        entry_point_contract.address.to_string(),
        Some(entry_point_contract.code_hash),
        balance.amount,
        None,
        None,
        None,
        0,
        token_contract.code_hash.clone(),
        token_contract.address.to_string(),
    ) {
        Ok(msg) => msg,
        Err(e) => return Err(ContractError::Std(e)),
    };

    Ok(Response::new()
        .add_message(send_msg)
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

    let asset_out = simulate_swap_exact_asset_in(deps, asset_in, swap_operations)?;

    // Return the asset out
    Ok(asset_out)
}

// Simulates a swap exact amount in request, returning the asset out and optionally the reverse simulation responses
fn simulate_swap_exact_asset_in(
    deps: Deps,
    asset_in: Asset,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Asset> {
    // Load state from storage
    let shade_pool_code_hash = SHADE_POOL_CODE_HASH.load(deps.storage)?;
    let shade_router_contract = SHADE_ROUTER_CONTRACT.load(deps.storage)?;

    // Get contract data for asset_in
    let asset_in_contract =
        REGISTERED_TOKENS.load(deps.storage, deps.api.addr_validate(asset_in.denom())?)?;

    let denom_out = match swap_operations.last() {
        Some(last_op) => last_op.denom_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    let mut path = vec![];
    for operation in swap_operations.iter() {
        path.push(shade_router::Hop {
            addr: operation.pool.to_string(),
            code_hash: shade_pool_code_hash.clone(),
        });
    }

    let sim_response: shade_router::QueryMsgResponse = deps.querier.query_wasm_smart(
        &shade_router_contract.address,
        &shade_router_contract.code_hash,
        &shade_router::QueryMsg::SwapSimulation {
            offer: shade_router::TokenAmount {
                token: shade_router::TokenType::CustomToken {
                    contract_addr: deps.api.addr_validate(asset_in.denom())?,
                    token_code_hash: asset_in_contract.code_hash,
                },
                amount: Uint128::new(asset_in.amount().u128()),
            },
            path,
            exclude_fee: None,
        },
    )?;

    let amount_out = match sim_response {
        shade_router::QueryMsgResponse::SwapSimulation { result, .. } => result.return_amount,
    };

    Ok(Asset::Cw20(Cw20Coin {
        address: denom_out.to_string(),
        amount: amount_out.u128().into(),
    }))
}
