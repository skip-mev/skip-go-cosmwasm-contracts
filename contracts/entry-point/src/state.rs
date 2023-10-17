use crate::reply::RecoverTempStorage;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const BLOCKED_CONTRACT_ADDRESSES: Map<&Addr, ()> = Map::new("blocked_contract_addresses");
pub const SWAP_VENUE_MAP: Map<&str, Addr> = Map::new("swap_venue_map");
pub const IBC_TRANSFER_CONTRACT_ADDRESS: Item<Addr> = Item::new("ibc_transfer_contract_address");

// Temporary state to save variables to be used in
// reply handling in case of recovering from an error
pub const RECOVER_TEMP_STORAGE: Item<RecoverTempStorage> = Item::new("recover_temp_storage");
