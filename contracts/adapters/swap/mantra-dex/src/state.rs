use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const MANTRA_DEX_POOL_MANAGER_ADDRESS: Item<Addr> =
    Item::new("mantra_dex_pool_manager_address");
