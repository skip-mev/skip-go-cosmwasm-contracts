use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/////////////
/// STATE ///
/////////////
pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("e");
pub const ROUTER_CONTRACT_ADDRESS: Item<Addr> = Item::new("r");
