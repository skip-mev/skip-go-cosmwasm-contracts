use cosmwasm_std::{entry_point, DepsMut, Empty, Env, MessageInfo, Response, StdError};
use cw2::set_contract_version;

///////////////////
/// INSTANTIATE ///
///////////////////

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, StdError> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}
