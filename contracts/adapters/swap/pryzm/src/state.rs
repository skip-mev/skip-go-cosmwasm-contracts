use crate::execution::SwapExecutionStep;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use std::collections::VecDeque;

pub const ENTRY_POINT_CONTRACT_ADDRESS: Item<Addr> = Item::new("entry_point_contract_address");

// stores the list of operations of the in progress swap, used by the reply entrypoint
pub const IN_PROGRESS_SWAP_OPERATIONS: Item<VecDeque<SwapExecutionStep>> =
    Item::new("in_progress_swap_operations");

// stores the address of the swapper for the in progress swap, used by the reply entrypoint
pub const IN_PROGRESS_SWAP_SENDER: Item<Addr> = Item::new("in_progress_swap_sender");
