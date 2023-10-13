use crate::error::ContractError;
use crate::state::SWAP_AND_ACTION_REQUEST_TEMP_STORAGE;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Env, Reply, Response, SubMsgResult};
use skip::entry_point::{Action, Affiliate};
use skip::swap::Swap;

pub const SWAP_AND_ACTION_REQUEST_REPLY_ID: u64 = 1;

#[cw_serde]
pub struct SwapActionTempStorage {
    pub user_swap: Swap,
    pub min_coin: Coin,
    pub timeout_timestamp: u64,
    pub post_swap_action: Action,
    pub affiliates: Vec<Affiliate>,
    pub funds: Vec<Coin>,
    pub recovery_addr: Addr,
}

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

pub fn handle_swap_and_action_request(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
     match msg.result {
        SubMsgResult::Ok(_response) => {
            SWAP_AND_ACTION_REQUEST_TEMP_STORAGE.remove(deps.storage);

            let res = Response::new().add_attribute("status", "swap_and_action_successful");

            Ok(res)
        }
        SubMsgResult::Err(e) => {
            let storage = SWAP_AND_ACTION_REQUEST_TEMP_STORAGE.load(deps.storage)?;
            let funds = storage.funds;

            // send original funds back to user
            let return_funds_msg = BankMsg::Send {
                to_address: storage.recovery_addr.to_string(),
                amount: funds,
            };

            SWAP_AND_ACTION_REQUEST_TEMP_STORAGE.remove(deps.storage);

            let res = Response::new()
                .add_message(return_funds_msg)
                .add_attribute("status", "swap_and_action_failed")
                .add_attribute("error", e);

            Ok(res)
        }
    }
}
