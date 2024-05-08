use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const HALLSWAP_CONTRACT_ADDRESS: Item<Addr> = Item::new("hallswap_contract_address");
