use crate::state::{IBC_TRANSFER_CONTRACT, SWAP_VENUE_MAP};
use cosmwasm_std::{Addr, Deps, StdResult};

// Queries the swap venue map by name and returns the swap adapter contract address if it exists
pub fn query_swap_venue_adapter_contract(deps: Deps, name: String) -> StdResult<Addr> {
    Ok(SWAP_VENUE_MAP.load(deps.storage, &name)?.address)
}

// Queries the IBC transfer adapter contract address and returns it if it exists
pub fn query_ibc_transfer_adapter_contract(deps: Deps) -> StdResult<Addr> {
    Ok(IBC_TRANSFER_CONTRACT.load(deps.storage)?.address)
}
