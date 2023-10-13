use crate::reply::SwapActionTempStorage;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const BLOCKED_CONTRACT_ADDRESSES: Map<&Addr, ()> = Map::new("blocked_contract_addresses");
pub const SWAP_VENUE_MAP: Map<&str, Addr> = Map::new("swap_venue_map");
pub const IBC_TRANSFER_CONTRACT_ADDRESS: Item<Addr> = Item::new("ibc_transfer_contract_address");

// Temporary state to save variables to be used on reply handling
pub const SWAP_AND_ACTION_REQUEST_TEMP_STORAGE: Item<SwapActionTempStorage> =
    Item::new("swap_and_action_request_temp_var");
