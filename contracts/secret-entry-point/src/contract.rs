use crate::{
    error::{ContractError, ContractResult},
    execute::{
        execute_action, execute_action_with_recover, execute_post_swap_action,
        execute_swap_and_action, execute_swap_and_action_with_recover, execute_user_swap,
        receive_snip20,
    },
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    query::{query_ibc_transfer_adapter_contract, query_swap_venue_adapter_contract},
    reply::{reply_swap_and_action_with_recover, RECOVER_REPLY_ID},
    state::{
        BLOCKED_CONTRACT_ADDRESSES, HYPERLANE_TRANSFER_CONTRACT_ADDRESS,
        IBC_TRANSFER_CONTRACT_ADDRESS, REGISTERED_TOKENS, SWAP_VENUE_MAP, VIEWING_KEY,
    },
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, ContractInfo, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
};
use secret_toolkit::snip20;
// use cw2::set_contract_version;

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
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Create response object to return
    let mut response: Response = Response::new().add_attribute("action", "instantiate");

    // Insert the entry point contract address into the blocked contract addresses map
    BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &env.contract.address, &())?;
    VIEWING_KEY.save(deps.storage, &msg.viewing_key)?;

    // Iterate through the swap venues provided and create a map of venue names to swap adapter contract addresses
    for swap_venue in msg.swap_venues.iter() {
        // Validate the swap contract address
        let checked_swap_contract = ContractInfo {
            address: deps
                .api
                .addr_validate(&swap_venue.adapter_contract.address.to_string())?,
            code_hash: swap_venue.adapter_contract.code_hash.clone(),
        };

        // Prevent duplicate swap venues by erroring if the venue name is already stored
        if SWAP_VENUE_MAP.has(deps.storage, &swap_venue.name) {
            return Err(ContractError::DuplicateSwapVenueName);
        }

        // Store the swap venue name and contract address inside the swap venue map
        SWAP_VENUE_MAP.save(deps.storage, &swap_venue.name, &checked_swap_contract)?;

        // Insert the swap contract address into the blocked contract addresses map
        BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &checked_swap_contract.address, &())?;

        // Add the swap venue and contract address to the response
        response = response
            .add_attribute("action", "add_swap_venue")
            .add_attribute("name", &swap_venue.name)
            .add_attribute("contract_address", &checked_swap_contract.address);
    }

    // Validate ibc transfer adapter contract addresses
    let checked_ibc_transfer_contract = ContractInfo {
        address: deps
            .api
            .addr_validate(&msg.ibc_transfer_contract.address.to_string())?,
        code_hash: msg.ibc_transfer_contract.code_hash.clone(),
    };

    // Store the ibc transfer adapter contract address
    IBC_TRANSFER_CONTRACT_ADDRESS.save(deps.storage, &checked_ibc_transfer_contract)?;

    // Insert the ibc transfer adapter contract address into the blocked contract addresses map
    BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &checked_ibc_transfer_contract.address, &())?;

    // Add the ibc transfer adapter contract address to the response
    response = response
        .add_attribute("action", "add_ibc_transfer_adapter")
        .add_attribute("contract_address", &checked_ibc_transfer_contract.address);

    // If the hyperlane transfer contract address is provided, validate and store it
    if let Some(hyperlane_transfer_contract) = msg.hyperlane_transfer_contract {
        // Validate hyperlane transfer adapter contract address
        let checked_hyperlane_transfer_contract = ContractInfo {
            address: deps
                .api
                .addr_validate(&hyperlane_transfer_contract.address.to_string())?,
            code_hash: hyperlane_transfer_contract.code_hash.clone(),
        };

        // Store the hyperlane transfer adapter contract address
        HYPERLANE_TRANSFER_CONTRACT_ADDRESS
            .save(deps.storage, &checked_hyperlane_transfer_contract)?;

        // Insert the hyperlane transfer adapter contract address into the blocked contract addresses map
        BLOCKED_CONTRACT_ADDRESSES.save(
            deps.storage,
            &checked_hyperlane_transfer_contract.address,
            &(),
        )?;

        // Add the hyperlane transfer adapter contract address to the response
        response = response
            .add_attribute("action", "add_hyperlane_transfer_adapter")
            .add_attribute(
                "contract_address",
                &checked_hyperlane_transfer_contract.address,
            );
    }

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
        ExecuteMsg::RegisterTokens { contracts } => register_tokens(deps, env, contracts),
        ExecuteMsg::Receive(msg) => receive_snip20(deps, env, info, msg),
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
        ExecuteMsg::Action {
            sent_asset,
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
        } => execute_action(
            deps,
            env,
            info,
            sent_asset,
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
        ),
        ExecuteMsg::ActionWithRecover {
            sent_asset,
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
            recovery_addr,
        } => execute_action_with_recover(
            deps,
            env,
            info,
            sent_asset,
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
            recovery_addr,
        ),
    }
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
            to_binary(&query_swap_venue_adapter_contract(deps, name)?)
        }
        QueryMsg::IbcTransferAdapterContract {} => {
            to_binary(&query_ibc_transfer_adapter_contract(deps)?)
        }
    }
}
