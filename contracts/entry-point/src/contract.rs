use crate::{
    error::{ContractError, ContractResult},
    execute::{execute_post_swap_action, execute_swap_and_action, execute_user_swap},
    query::{query_ibc_transfer_adapter_contract, query_swap_venue_adapter_contract},
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS, SWAP_VENUE_MAP},
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use skip::entry_point::{ExecuteMsg, InstantiateMsg, QueryMsg};

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Create response object to return
    let mut response: Response = Response::new().add_attribute("action", "instantiate");

    // Insert the entry point contract address into the blocked contract addresses map
    BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &env.contract.address, &())?;

    // Iterate through the swap venues provided and create a map of venue names to swap adapter contract addresses
    for swap_venue in msg.swap_venues.iter() {
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

    // Validate ibc transfer adapter contract addresses
    let checked_ibc_transfer_contract_address =
        deps.api.addr_validate(&msg.ibc_transfer_contract_address)?;

    // Store the ibc transfer adapter contract address
    IBC_TRANSFER_CONTRACT_ADDRESS.save(deps.storage, &checked_ibc_transfer_contract_address)?;

    // Insert the ibc transfer adapter contract address into the blocked contract addresses map
    BLOCKED_CONTRACT_ADDRESSES.save(deps.storage, &checked_ibc_transfer_contract_address, &())?;

    // Add the ibc transfer adapter contract address to the response
    response = response
        .add_attribute("action", "add_ibc_transfer_adapter")
        .add_attribute("contract_address", &checked_ibc_transfer_contract_address);

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
        ExecuteMsg::SwapAndAction {
            fee_swap,
            user_swap,
            min_coin,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        } => execute_swap_and_action(
            deps,
            env,
            info,
            fee_swap,
            user_swap,
            min_coin,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        ),
        ExecuteMsg::UserSwap {
            swap, 
            min_coin, 
            affiliates,
        } => execute_user_swap(
            deps,
            env,
            info,
            swap,
            min_coin,
            affiliates,
        ),
        ExecuteMsg::PostSwapAction {
            min_coin,
            timeout_timestamp,
            post_swap_action,
        } => execute_post_swap_action(
            deps,
            env,
            info,
            min_coin,
            timeout_timestamp,
            post_swap_action,
        ),
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
