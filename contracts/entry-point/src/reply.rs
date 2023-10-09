use crate::error::ContractError;
use crate::state::{ACTION_REQUEST_TEMP_STORAGE, SWAP_REQUEST_TEMP_STORAGE};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Env, Reply, Response, SubMsgResult};
use skip::entry_point::Action;
use skip::swap::Swap;

pub const USER_SWAP_REQUEST_REPLY_ID: u64 = 0;
pub const SWAP_REQUEST_REPLY_ID: u64 = 1;
pub const ACTION_REQUEST_REPLY_ID: u64 = 2;

#[cw_serde]
pub struct SwapTempStorage {
    pub recovery_addr: Addr,
    pub funds: Vec<Coin>,
    pub swap: Swap,
}

#[cw_serde]
pub struct ActionTempStorage {
    pub recovery_addr: Addr,
    pub funds: Vec<Coin>,
    pub action: Action,
}

pub fn handle_swap_request(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    return match msg.result {
        SubMsgResult::Ok(_response) => {
            SWAP_REQUEST_TEMP_STORAGE.remove(deps.storage);

            let res = Response::new().add_attribute("status", "swap_successful");

            Ok(res)
        }
        SubMsgResult::Err(e) => {
            let storage = SWAP_REQUEST_TEMP_STORAGE.load(deps.storage)?;
            let funds = storage.funds;

            // send original funds back to user
            let return_funds_msg = BankMsg::Send {
                to_address: storage.recovery_addr.to_string(),
                amount: funds,
            };

            SWAP_REQUEST_TEMP_STORAGE.remove(deps.storage);

            let res = Response::new()
                .add_message(return_funds_msg)
                .add_attribute("status", "swap_failed")
                .add_attribute("error", e);

            Ok(res)
        }
    };
}

pub fn handle_action_request(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    return match msg.result {
        SubMsgResult::Ok(_response) => {
            ACTION_REQUEST_TEMP_STORAGE.remove(deps.storage);

            let res = Response::new().add_attribute("status", "action_successful");

            Ok(res)
        }
        SubMsgResult::Err(e) => {
            let storage = ACTION_REQUEST_TEMP_STORAGE.load(deps.storage)?;
            let funds = storage.funds;

            let return_funds_msg = BankMsg::Send {
                to_address: storage.recovery_addr.to_string(),
                amount: funds,
            };

            ACTION_REQUEST_TEMP_STORAGE.remove(deps.storage);

            let res = Response::new()
                .add_message(return_funds_msg)
                .add_attribute("status", "action_failed")
                .add_attribute("error", e.to_string());

            Ok(res)
        }
    };
}
