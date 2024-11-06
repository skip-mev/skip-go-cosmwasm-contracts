use crate::reply::RecoverTempStorage;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

pub const BLOCKED_CONTRACT_ADDRESSES: Map<&Addr, ()> = Map::new("blocked_contract_addresses");
pub const SWAP_VENUE_MAP: Map<&str, Addr> = Map::new("swap_venue_map");
pub const IBC_TRANSFER_CONTRACT_ADDRESS: Item<Addr> = Item::new("ibc_transfer_contract_address");
pub const HYPERLANE_TRANSFER_CONTRACT_ADDRESS: Item<Addr> =
    Item::new("hyperlane_transfer_contract_address");

// Temporary state to save variables to be used in
// reply handling in case of recovering from an error
pub const RECOVER_TEMP_STORAGE: Item<RecoverTempStorage> = Item::new("recover_temp_storage");

// Temporary state to save the amount of the out asset the contract
// has pre swap so that we can ensure the amount transferred out does not
// exceed the amount the contract obtained from the current swap/call
pub const PRE_SWAP_OUT_ASSET_AMOUNT: Item<Uint128> = Item::new("pre_swap_out_asset_amount");
