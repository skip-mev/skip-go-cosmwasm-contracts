use cosmwasm_std::{Addr, ContractInfo};
use secret_skip::ibc::AckID;
use secret_storage_plus::{Item, Map};

pub const ENTRY_POINT_CONTRACT: Item<ContractInfo> = Item::new("entry_point_contract_address");
pub const IN_PROGRESS_RECOVER_ADDRESS: Item<String> = Item::new("in_progress_recover_address");
pub const IN_PROGRESS_CHANNEL_ID: Item<String> = Item::new("in_progress_channel_id");
pub const ACK_ID_TO_RECOVER_ADDRESS: Map<AckID, String> = Map::new("ack_id_to_recover_address");

pub const ICS20_CONTRACT: Item<ContractInfo> = Item::new("ics20_contract");

pub const REGISTERED_TOKENS: Map<Addr, ContractInfo> = Map::new("registered_tokens");

pub const VIEWING_KEY: Item<String> = Item::new("viewing_key");
