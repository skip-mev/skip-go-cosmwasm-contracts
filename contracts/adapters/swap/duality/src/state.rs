use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const DEX_MODULE_ADDRESS: Item<Addr> = Item::new("dex_module_address");
