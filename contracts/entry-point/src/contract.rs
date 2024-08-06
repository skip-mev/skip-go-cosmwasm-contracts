use crate::{
    error::{ContractError, ContractResult},
    execute::{
        execute_post_swap_action, execute_swap_and_action, execute_swap_and_action_with_recover,
        execute_universal_swap, execute_update_config, execute_user_swap, receive_cw20,
    },
    query::{query_ibc_transfer_adapter_contract, query_swap_venue_adapter_contract},
    reply::{reply_swap_and_action_with_recover, RECOVER_REPLY_ID},
    state::{
        BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS, IBC_WASM_CONTRACT_ADDRESS,
        OWNER, SWAP_VENUE_MAP,
    },
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
};
use cw2::set_contract_version;
use skip::entry_point::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    Ok(Response::default())
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
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Create response object to return
    let mut response: Response = Response::new().add_attribute("action", "instantiate");

    // Insert the entry point contract address into the blocked contract addresses map
    BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &env.contract.address, &())?;

    // Iterate through the swap venues provided and create a map of venue names to swap adapter contract addresses

    if let Some(swap_venues) = msg.swap_venues {
        for swap_venue in swap_venues.iter() {
            // Validate the swap contract address
            let checked_swap_contract_address = deps
                .api
                .addr_validate(&swap_venue.adapter_contract_address)?;

            // Prevent duplicate swap venues by erroring if the venue name is already stored
            if SWAP_VENUE_MAP.has(deps.storage, &swap_venue.name) {
                return Err(ContractError::DuplicateSwapVenueName);
            }

            // Store the swap venue name and contract address inside the swap venue map
            SWAP_VENUE_MAP.save(
                deps.storage,
                &swap_venue.name,
                &checked_swap_contract_address,
            )?;

            // Insert the swap contract address into the blocked contract addresses map
            BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &checked_swap_contract_address, &())?;

            // Add the swap venue and contract address to the response
            response = response
                .add_attribute("action", "add_swap_venue")
                .add_attribute("name", &swap_venue.name)
                .add_attribute("contract_address", &checked_swap_contract_address);
        }
    }

    if let Some(ibc_transfer_contract_address) = msg.ibc_transfer_contract_address {
        // Validate ibc transfer adapter contract addresses
        let checked_ibc_transfer_contract_address =
            deps.api.addr_validate(&ibc_transfer_contract_address)?;

        // Store the ibc transfer adapter contract address
        IBC_TRANSFER_CONTRACT_ADDRESS.save(deps.storage, &checked_ibc_transfer_contract_address)?;

        // Insert the ibc transfer adapter contract address into the blocked contract addresses map
        BLOCKED_CONTRACT_ADDRESSES.save(
            deps.storage,
            &checked_ibc_transfer_contract_address,
            &(),
        )?;

        // Add the ibc transfer adapter contract address to the response
        response = response
            .add_attribute("action", "add_ibc_transfer_adapter")
            .add_attribute("contract_address", &checked_ibc_transfer_contract_address);
    }

    if let Some(ibc_wasm_contract_address) = msg.ibc_wasm_contract_address {
        // Validate ibc transfer adapter contract addresses
        let checked_ibc_wasm_contract_address =
            deps.api.addr_validate(&ibc_wasm_contract_address)?;

        // Store the ibc transfer adapter contract address
        IBC_WASM_CONTRACT_ADDRESS.save(deps.storage, &checked_ibc_wasm_contract_address)?;

        // Insert the ibc transfer adapter contract address into the blocked contract addresses map
        BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &checked_ibc_wasm_contract_address, &())?;

        // Add the ibc transfer adapter contract address to the response
        response = response
            .add_attribute("action", "add_ibc_wasm_transfer_adapter")
            .add_attribute("contract_address", &checked_ibc_wasm_contract_address);
    }

    // Store sender as owner
    OWNER.set(deps, Some(info.sender))?;

    Ok(response)
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
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::SwapAndActionWithRecover {
            sent_asset,
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
            recovery_addr,
        } => execute_swap_and_action_with_recover(
            deps,
            env,
            info,
            sent_asset,
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
            recovery_addr,
        ),
        ExecuteMsg::SwapAndAction {
            sent_asset,
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        } => execute_swap_and_action(
            deps,
            env,
            info,
            sent_asset,
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        ),
        ExecuteMsg::UserSwap {
            swap,
            min_asset,
            remaining_asset,
            affiliates,
        } => execute_user_swap(
            deps,
            env,
            info,
            swap,
            min_asset,
            remaining_asset,
            affiliates,
        ),
        ExecuteMsg::PostSwapAction {
            min_asset,
            timeout_timestamp,
            post_swap_action,
            exact_out,
        } => execute_post_swap_action(
            deps,
            env,
            info,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            exact_out,
        ),
        ExecuteMsg::UpdateConfig {
            owner,
            swap_venues,
            ibc_transfer_contract_address,
            ibc_wasm_contract_address,
        } => execute_update_config(
            deps,
            info,
            owner,
            swap_venues,
            ibc_transfer_contract_address,
            ibc_wasm_contract_address,
        ),
        ExecuteMsg::UniversalSwap { memo } => execute_universal_swap(deps, env, info, None, memo),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        RECOVER_REPLY_ID => reply_swap_and_action_with_recover(deps, msg),
        _ => Err(ContractError::ReplyIdError(msg.id)),
    }
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::SwapVenueAdapterContract { name } => {
            to_json_binary(&query_swap_venue_adapter_contract(deps, name)?)
        }
        QueryMsg::IbcTransferAdapterContract {} => {
            to_json_binary(&query_ibc_transfer_adapter_contract(deps)?)
        }
    }
}
