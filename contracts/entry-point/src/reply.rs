use crate::error::ContractError;
use crate::state::RECOVER_TEMP_STORAGE;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Reply, Response, SubMsgResult};

pub const RECOVER_REPLY_ID: u64 = 1;

#[cw_serde]
pub struct RecoverTempStorage {
    pub funds: Vec<Coin>,
    pub recovery_addr: Addr,
}

pub fn reply_swap_and_action_with_recover(
    deps: DepsMut,
    msg: Reply,
) -> Result<Response, ContractError> {
    match msg.result {
        SubMsgResult::Ok(_response) => {
            RECOVER_TEMP_STORAGE.remove(deps.storage);

            Ok(Response::new().add_attribute("status", "swap_and_action_successful"))
        }
        SubMsgResult::Err(e) => {
            let storage = RECOVER_TEMP_STORAGE.load(deps.storage)?;
            let funds = storage.funds;

            // send original funds back to user
            let return_funds_msg = BankMsg::Send {
                to_address: storage.recovery_addr.to_string(),
                amount: funds,
            };

            RECOVER_TEMP_STORAGE.remove(deps.storage);

            Ok(Response::new()
                .add_message(return_funds_msg)
                .add_attribute("status", "swap_and_action_failed")
                .add_attribute("error", e))
        }
    }
}
