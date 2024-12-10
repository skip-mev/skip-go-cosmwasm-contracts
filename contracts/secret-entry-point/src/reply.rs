use crate::error::ContractError;
use crate::state::RECOVER_TEMP_STORAGE;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Reply, Response, SubMsgResult};
use skip::asset::Asset;

pub const RECOVER_REPLY_ID: u64 = 1;

#[cw_serde]
pub struct RecoverTempStorage {
    pub assets: Vec<Asset>,
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

            let return_assets_msgs: Vec<CosmosMsg> = storage
                .assets
                .into_iter()
                .map(|asset| asset.transfer(storage.recovery_addr.as_str()))
                .collect();

            RECOVER_TEMP_STORAGE.remove(deps.storage);

            Ok(Response::new()
                .add_messages(return_assets_msgs)
                .add_attribute("status", "swap_and_action_failed")
                .add_attribute("error", e))
        }
    }
}
