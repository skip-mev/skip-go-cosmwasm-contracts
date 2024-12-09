use crate::contract::State;
use cosmwasm_std::{Addr, ContractInfo};
use secret_storage_plus::Map;
use secret_toolkit::storage::Item;

pub const STATE: Item<State> = Item::new(b"state");
/*
pub const ENTRY_POINT_CONTRACT: Item<ContractInfo> = Item::new(b"entry_point_contract");

pub const SHADE_ROUTER_CONTRACT: Item<ContractInfo> = Item::new(b"shade_router_contract");

pub const SHADE_POOL_CODE_HASH: Item<String> = Item::new(b"shade_pair_code_hash");

pub const VIEWING_KEY: Item<String> = Item::new(b"viewing_key");
*/

pub const REGISTERED_TOKENS: Map<Addr, ContractInfo> = Map::new("registered_tokens");
