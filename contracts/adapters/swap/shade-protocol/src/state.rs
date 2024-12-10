use crate::contract::State;
use cosmwasm_std::{Addr, ContractInfo};
use secret_storage_plus::Map;
use secret_toolkit::storage::Item;

pub const STATE: Item<State> = Item::new(b"state");

pub const REGISTERED_TOKENS: Map<Addr, ContractInfo> = Map::new("registered_tokens");
