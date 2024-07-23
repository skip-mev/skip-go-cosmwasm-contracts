use crate::{
    error::{ContractError, ContractResult},
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IBC_WASM_CONTRACT_ADDRESS},
};
use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, SubMsg, SubMsgResult,
};
use cw2::set_contract_version;

use skip::ibc_wasm::{ExecuteMsg, IbcInfo, InstantiateMsg, MigrateMsg, QueryMsg};

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
    // Validate ibc bridge wasm contract address
    let checked_ibc_wasm_contract_address =
        deps.api.addr_validate(&msg.ibc_wasm_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;
    IBC_WASM_CONTRACT_ADDRESS.save(deps.storage, &checked_ibc_wasm_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "ibc_wasm_contract_address",
            checked_ibc_wasm_contract_address.to_string(),
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
        ExecuteMsg::IbcWasmTransfer {
            info: ibc_info,
            coin,
            timeout_timestamp,
        } => execute_ibc_wasm_transfer(deps, env, info, ibc_info, coin, timeout_timestamp),
    }
}

// Converts the given info and coin into a  ibc wasm transfer message,
fn execute_ibc_wasm_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ibc_info: IbcInfo,
    coin: Coin,
    timeout_timestamp: u64,
) -> ContractResult<Response> {
    // TODO
    // validate coin

    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Error if ibc_info.fee is not Some since they are required on Neutron.
    let ibc_fee = match ibc_info.fee {
        Some(fee) => fee,
        None => return Err(ContractError::IbcFeesRequired),
    };

    Ok(Response::new()
        // .add_submessage(sub_msg)
        .add_attribute("action", "execute_ibc_wasm_transfer"))
}
