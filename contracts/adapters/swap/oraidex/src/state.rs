use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const DEXTER_VAULT_ADDRESS: Item<Addr> = Item::new("dexter_vault_address");
pub const DEXTER_ROUTER_ADDRESS: Item<Addr> = Item::new("dexter_router_address");
