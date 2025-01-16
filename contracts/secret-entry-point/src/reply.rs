use crate::{
    error::ContractError,
    state::{RECOVER_TEMP_STORAGE, REGISTERED_TOKENS},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Reply, Response, SubMsgResult};
use secret_skip::asset::Asset;
use secret_toolkit::snip20;

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

            let mut return_assets_msgs: Vec<CosmosMsg> = vec![];

            for return_asset in storage.assets.into_iter() {
                let return_asset_contract = REGISTERED_TOKENS
                    .load(deps.storage, deps.api.addr_validate(return_asset.denom())?)?;
                match snip20::transfer_msg(
                    storage.recovery_addr.to_string(),
                    return_asset.amount(),
                    None,
                    None,
                    0,
                    return_asset_contract.code_hash.clone(),
                    return_asset_contract.address.to_string(),
                ) {
                    Ok(msg) => return_assets_msgs.push(msg),
                    Err(e) => return Err(ContractError::Std(e)),
                };
            }

            RECOVER_TEMP_STORAGE.remove(deps.storage);

            Ok(Response::new()
                .add_messages(return_assets_msgs)
                .add_attribute("status", "swap_and_action_failed")
                .add_attribute("error", e))
        }
    }
}
