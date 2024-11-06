use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{entry_point, to_json_binary, DepsMut, Env, HexBinary, MessageInfo, Response};
use cw2::set_contract_version;
use cw_utils::one_coin;
use hpl_interface::warp::native::ExecuteMsg::TransferRemote;
use skip::{
    asset::Asset,
    hyperlane::{ExecuteMsg, InstantiateMsg, MigrateMsg},
};

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

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
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::HplTransfer {
            dest_domain,
            recipient,
            hook,
            metadata,
            warp_address,
        } => execute_hpl_transfer(
            deps,
            info,
            dest_domain,
            recipient,
            hook,
            metadata,
            warp_address,
        ),
    }
}

// Converts the given info and coin into a Hyperlane remote transfer
fn execute_hpl_transfer(
    deps: DepsMut,
    info: MessageInfo,
    dest_domain: u32,
    recipient: HexBinary,
    hook: Option<String>,
    metadata: Option<HexBinary>,
    warp_address: String,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get the asset from the info
    let asset: Asset = one_coin(&info)?.into();

    // Create the Hyperlane remote transfer message
    let msg = to_json_binary(&TransferRemote {
        dest_domain,
        recipient,
        amount: asset.amount(),
        hook,
        metadata,
    })?;

    // Convert the hyperlane transfer message into a wasm message
    let hpl_msg = asset.into_wasm_msg(warp_address, msg)?;

    Ok(Response::new()
        .add_message(hpl_msg)
        .add_attribute("action", "execute_hyperlane_transfer"))
}
