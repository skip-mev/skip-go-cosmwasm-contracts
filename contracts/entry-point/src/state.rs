use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const SWAP_VENUE_MAP: Map<&str, Addr> = Map::new("swap_venue_map");
pub const IBC_TRANSFER_CONTRACT_ADDRESS: Item<Addr> = Item::new("ibc_transfer_contract_address");
