use crate::{
    error::{ContractError, ContractResult},
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IBC_WASM_CONTRACT_ADDRESS},
};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw20_ics20_msg::msg::TransferBackMsg;
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    ibc_wasm::{Cw20HookMsg, ExecuteMsg, IbcWasmExecuteMsg, InstantiateMsg, MigrateMsg},
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

    // Set the sender to the originating address that triggered the cw20 send call
    // This is later validated / enforced to be the entry point contract address
    info.sender = deps.api.addr_validate(&cw20_msg.sender)?;

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::IbcWasmTransfer {
            ibc_wasm_info,
            coin,
        } => {
            if sent_asset.ne(&coin) {
                return Err(ContractError::InvalidFund {});
            }
            execute_ibc_wasm_transfer(deps, env, info, ibc_wasm_info, coin)
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
        ExecuteMsg::IbcWasmTransfer {
            ibc_wasm_info,
            coin,
        } => {
            let fund = one_coin(&info)?;
            if Asset::from(fund).ne(&coin) {
                return Err(ContractError::InvalidFund {});
            }
            execute_ibc_wasm_transfer(deps, env, info, ibc_wasm_info, coin)
        }
    }
}

// Converts the given info and coin into a  ibc wasm transfer message,
fn execute_ibc_wasm_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    transfer_back_msg: TransferBackMsg,
    asset: Asset,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    let ibc_wasm_contract = IBC_WASM_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    let msg = match &asset {
        Asset::Native(coin) => WasmMsg::Execute {
            contract_addr: ibc_wasm_contract.to_string(),
            msg: to_json_binary(&IbcWasmExecuteMsg::TransferToRemote(transfer_back_msg))?,
            funds: vec![coin.clone()],
        },
        Asset::Cw20(coin) => WasmMsg::Execute {
            contract_addr: coin.address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: ibc_wasm_contract.to_string(),
                amount: coin.amount,
                msg: to_json_binary(&transfer_back_msg)?,
            })?,
            funds: vec![],
        },
    };

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "execute_ibc_wasm_transfer"))
}
