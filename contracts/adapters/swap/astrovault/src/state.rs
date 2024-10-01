use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const ASTROVAULT_ROUTER_ADDRESS: Item<Addr> = Item::new("astrovault_router_address");
pub const ASTROVAULT_CASHBACK_ADDRESS: Item<Addr> = Item::new("astrovault_cashback_address");
