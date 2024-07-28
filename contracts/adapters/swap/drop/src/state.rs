use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const DROP_CORE_CONTRACT_ADDRESS: Item<Addr> = Item::new("drop_core_contract_address");

pub const BONDED_DENOM: Item<String> = Item::new("bonded_denom");
pub const REMOTE_DENOM: Item<String> = Item::new("remote_denom");
