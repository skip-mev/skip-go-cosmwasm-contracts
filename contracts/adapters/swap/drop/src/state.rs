use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");
pub const DROP_CORE_CONTRACT_ADDRESS: Item<Addr> = Item::new("drop_core_contract_address");
pub const DROP_TOKEN_CONTRACT_ADDRESS: Item<Addr> = Item::new("drop_token_contract_address");

pub const FACTORY_BONDED_DENOM: Item<String> = Item::new("factory_bonded_denom");
pub const IBC_REMOTE_DENOM: Item<String> = Item::new("ibc_remote_denom");
