use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ROUTER_CONTRACT_ADDRESS: Item<Addr> = Item::new("router_contract_address");
