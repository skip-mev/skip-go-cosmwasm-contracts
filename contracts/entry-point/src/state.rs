use crate::reply::{ActionTempStorage, SwapTempStorage};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const BLOCKED_CONTRACT_ADDRESSES: Map<&Addr, ()> = Map::new("blocked_contract_addresses");
pub const SWAP_VENUE_MAP: Map<&str, Addr> = Map::new("swap_venue_map");
pub const IBC_TRANSFER_CONTRACT_ADDRESS: Item<Addr> = Item::new("ibc_transfer_contract_address");

// Temporary state to save variables to be used on reply handling
pub const SWAP_REQUEST_TEMP_STORAGE: Item<SwapTempStorage> = Item::new("swap_request_temp_var");

// Temporary state to save variables to be used on reply handling
pub const ACTION_REQUEST_TEMP_STORAGE: Item<ActionTempStorage> =
    Item::new("action_request_temp_var");
