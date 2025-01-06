use crate::reply::RecoverTempStorage;
use cosmwasm_std::{Addr, ContractInfo, Uint128};
use secret_storage_plus::{Item, Map};

pub const BLOCKED_CONTRACT_ADDRESSES: Map<&Addr, ()> = Map::new("blocked_contract_addresses");
pub const SWAP_VENUE_MAP: Map<&str, ContractInfo> = Map::new("swap_venue_map");
pub const IBC_TRANSFER_CONTRACT: Item<ContractInfo> = Item::new("ibc_transfer_contract");
/*
pub const HYPERLANE_TRANSFER_CONTRACT_ADDRESS: Item<ContractInfo> =
    Item::new("hyperlane_transfer_contract_address");
*/

// Temporary state to save variables to be used in
// reply handling in case of recovering from an error
pub const RECOVER_TEMP_STORAGE: Item<RecoverTempStorage> = Item::new("recover_temp_storage");

// Temporary state to save the amount of the out asset the contract
// has pre swap so that we can ensure the amount transferred out does not
// exceed the amount the contract obtained from the current swap/call
pub const PRE_SWAP_OUT_ASSET_AMOUNT: Item<Uint128> = Item::new("pre_swap_out_asset_amount");

// Secret Network tokens need to be registered for viewing key setup
// and storing contract code hash
pub const REGISTERED_TOKENS: Map<Addr, ContractInfo> = Map::new("registered_tokens");

pub const VIEWING_KEY: Item<String> = Item::new("viewing_key");
