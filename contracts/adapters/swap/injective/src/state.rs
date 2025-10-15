use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const INJECTIVE_SWAP_CONTRACT_ADDRESS: Item<Addr> =
    Item::new("injective_swap_contract_address");
