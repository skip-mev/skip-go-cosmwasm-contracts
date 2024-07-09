use std::collections::VecDeque;

use cosmwasm_std::{
    Addr, Binary, Coin, Deps, DepsMut, entry_point, Env, MessageInfo, Reply, Response,
    SubMsg, SubMsgResponse, SubMsgResult, to_json_binary, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use pryzm_std::types::pryzm::{amm::v1::MsgBatchSwapResponse, icstaking::v1::MsgStakeResponse};

use skip::swap::{
    execute_transfer_funds_back, ExecuteMsg, get_ask_denom_for_routes, InstantiateMsg, MigrateMsg,
    QueryMsg, SwapOperation,
};

use crate::{
    error::{ContractError, ContractResult},
    execution::{extract_execution_steps, parse_coin, SwapExecutionStep},
    reply_id,
    simulate::{
        simulate_smart_swap_exact_asset_in_with_metadata, simulate_swap_exact_asset_in,
        simulate_swap_exact_asset_in_with_metadata, simulate_swap_exact_asset_out,
        simulate_swap_exact_asset_out_with_metadata, simulate_smart_swap_exact_asset_in,
    },
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_SWAP_OPERATIONS, IN_PROGRESS_SWAP_SENDER},
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

    // Extract the execution steps from the provided swap operations
    let execution_steps = extract_execution_steps(operations)?;

    // Execute the swap
    return execute_steps(deps, env, info.sender, coin_in, execution_steps);
}

// Executes the swap of the provided coin using the provided execution steps for the swapper
fn execute_steps(
    deps: DepsMut,
    env: Env,
    swapper: Addr,
    coin_in: Coin,
    execution_steps: VecDeque<SwapExecutionStep>,
) -> ContractResult<Response> {
    // return error if execution_steps is empty
    if execution_steps.is_empty() {
        return Err(ContractError::SwapOperationsEmpty);
    }

    // convert the first execution step to the appropriate cosmos message
    let first_step = execution_steps.front().unwrap();
    let msg = first_step
        .clone()
        .to_cosmos_msg(env.contract.address.to_string(), coin_in)?;

    // If there is only one execution step, create the transfer funds back message since the swap is done is a single step
    if execution_steps.len() == 1 {
        // Create the transfer funds back message
        let return_denom = first_step.clone().get_return_denom()?;
        let transfer_funds_back_msg = WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                swapper,
                return_denom,
            })?,
            funds: vec![],
        };

        return Ok(Response::new()
            .add_message(msg.clone())
            .add_message(transfer_funds_back_msg)
            .add_attribute("action", "dispatch_swap_and_transfer_back"));
    }

    // if there are more than one step, create sub message for the first step
    let sub_msg = match first_step {
        SwapExecutionStep::Swap { .. } => {
            SubMsg::reply_on_success(msg.clone(), reply_id::BATCH_SWAP_REPLY_ID)
        }
        SwapExecutionStep::Stake { .. } => {
            SubMsg::reply_on_success(msg.clone(), reply_id::STAKE_REPLY_ID)
        }
    };

    // store the steps to continue after the current step is executed in the reply entrypoint
    IN_PROGRESS_SWAP_OPERATIONS.save(deps.storage, &execution_steps)?;

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "dispatch_swap_step"))
}

/////////////
/// REPLY ///
/////////////

// Handles the reply from the swap step execution messages
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> ContractResult<Response> {
    // Get the sub message result from the reply
    let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = reply.result else {
        return Err(ContractError::InvalidState {
            msg: "could not get sub message response from reply result".to_string(),
        });
    };

    // handle the reply and use the output of the swap as the coin_in for the next swap steps
    let coin_in: Coin;
    match reply.id {
        reply_id::BATCH_SWAP_REPLY_ID => {
            // Parse the batch swap response from the sub message
            let resp: MsgBatchSwapResponse = b.try_into().map_err(ContractError::Std).unwrap();
            if resp.amounts_out.len() != 1 {
                return Err(ContractError::InvalidMsgResponse {
                    msg: "unexpected amounts out length is batch swap response".to_string(),
                });
            }
            coin_in = parse_coin(resp.amounts_out.first().unwrap())
        }
        reply_id::STAKE_REPLY_ID => {
            // Parse the stake response from the sub message
            let resp: MsgStakeResponse = b.try_into().map_err(ContractError::Std).unwrap();
            if let Some(c_amount) = resp.c_amount {
                coin_in = parse_coin(&c_amount)
            } else {
                return Err(ContractError::InvalidMsgResponse {
                    msg: "expected valid c_amount in stake response, received None".to_string(),
                });
            }
        }
        _ => {
            return Err(ContractError::InvalidState {
                msg: format!("unexpected reply id {}", reply.id),
            });
        }
    }

    // load the swap execution steps from the store
    let mut in_progress_exec_steps = IN_PROGRESS_SWAP_OPERATIONS.load(deps.storage)?;
    IN_PROGRESS_SWAP_OPERATIONS.remove(deps.storage);

    // load the swapper address from the store
    let swapper = IN_PROGRESS_SWAP_SENDER.load(deps.storage)?;
    IN_PROGRESS_SWAP_SENDER.remove(deps.storage);

    // remove the first step (which is already executed)
    in_progress_exec_steps.pop_front();

    // continue the swap execution with the next steps
    return execute_steps(deps, env, swapper, coin_in, in_progress_exec_steps);
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
        } => to_json_binary(&simulate_swap_exact_asset_in(
            deps,
            asset_in,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out,
            swap_operations,
        } => to_json_binary(&simulate_swap_exact_asset_out(
            deps,
            asset_out,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in,
            swap_operations,
            include_spot_price,
        } => to_json_binary(&simulate_swap_exact_asset_in_with_metadata(
            deps,
            asset_in,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out,
            swap_operations,
            include_spot_price,
        } => to_json_binary(&simulate_swap_exact_asset_out_with_metadata(
            deps,
            asset_out,
            swap_operations,
            include_spot_price,
        )?),
        QueryMsg::SimulateSmartSwapExactAssetIn { routes, .. } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_json_binary(&simulate_smart_swap_exact_asset_in(
                deps, ask_denom, routes,
            )?)
        }
        QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
            routes,
            asset_in,
            include_spot_price,
        } => {
            let ask_denom = get_ask_denom_for_routes(&routes)?;

            to_json_binary(&simulate_smart_swap_exact_asset_in_with_metadata(
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
