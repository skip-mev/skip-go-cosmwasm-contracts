use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");

pub const PRE_SWAP_OUT_ASSET_AMOUNT: Item<Uint128> = Item::new("pre_swap_out_asset_amount");
