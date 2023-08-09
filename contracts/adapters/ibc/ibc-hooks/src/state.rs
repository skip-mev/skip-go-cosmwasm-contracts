use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use skip::ibc::AckID;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const IN_PROGRESS_RECOVER_ADDRESS: Item<String> = Item::new("in_progress_recover_address");
pub const IN_PROGRESS_CHANNEL_ID: Item<String> = Item::new("in_progress_channel_id");
pub const ACK_ID_TO_RECOVER_ADDRESS: Map<AckID, String> = Map::new("ack_id_to_recover_address");
