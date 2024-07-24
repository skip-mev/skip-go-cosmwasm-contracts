use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const IBC_WASM_CONTRACT_ADDRESS: Item<Addr> = Item::new("checked_ibc_wasm_contract_address");
