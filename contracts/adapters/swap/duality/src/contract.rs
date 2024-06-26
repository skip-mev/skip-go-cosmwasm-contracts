use crate::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::one_coin;
use neutron_sdk::{bindings::dex::{types::{MultiHopRoute, PrecDec}, msg::DexMsg::MultiHopSwap}};
use skip::{
    asset::Asset,
    swap::{
        execute_transfer_funds_back, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryMsg, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse,
        SwapOperation,
    },
};
use std::str::FromStr;

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    unimplemented!()
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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Swap { operations } => execute_swap(deps, env, info, operations),
        ExecuteMsg::TransferFundsBack {
            swapper,
            return_denom,
        } => Ok(execute_transfer_funds_back(
            deps,
            env,
            info,
            swapper,
            return_denom,
        )?),
        _ => {
            unimplemented!()
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get coin in from the message info, error if there is not exactly one coin sent
    let coin_in = one_coin(&info)?;

    let return_denom = match operations.last() {
        Some(last_op) => last_op.denom_out.clone(),
        None => return Err(ContractError::SwapOperationsEmpty),
    };

    //build duality Swap message
    let swap_msg = create_duality_swap_msg(&env, coin_in,  operations)?;

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
            swapper: info.sender,
            return_denom,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

// Creates the duality swap message
fn create_duality_swap_msg(
    env: &Env,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<CosmosMsg> {

    // Convert the swap operations into a Duality multi hop swap route.
    let route = match get_route_from_swap_operations(swap_operations) {
        Ok(route) => route,
        Err(e) => return Err(e),
    };

    // Create the duality multi hop swap message
    let swap_msg: CosmosMsg = MultiHopSwap {
        receiver: env.contract.address.to_string(),
        routes: vec![route],
        amount_in: coin_in.amount.into(),
        exit_limit_price: PrecDec{i: "0.00000001".to_string()},
        pick_best_route: true,
    }
    .into();

    Ok(swap_msg)
}

// multi-hop-swap routes are a string array of denoms to route through 
// with formal [tokenA,tokenB,tokenC,tokenD]
pub fn get_route_from_swap_operations(
    swap_operations: Vec<SwapOperation>
) -> Result<neutron_sdk::bindings::dex::types::MultiHopRoute, String>  {
    if swap_operations.is_empty() {
        return Err(format!(
            "Empty Swap Operation",
        ));
    }

    let mut route = vec![swap_operations[0].denom_in.clone(), swap_operations[0].denom_out.clone()];
    let mut last_denom_out = &swap_operations[0].denom_out;

    for operation in swap_operations.iter().skip(1) {  
        if &operation.denom_in != last_denom_out {
            return Err(format!(
                "Mismatch: denom_in {} does not match last denom_out {}",
                operation.denom_in, last_denom_out
            ));
        }
        route.push(operation.denom_out.clone());
        last_denom_out = &operation.denom_out;
    }

    Ok(MultiHopRoute { hops: route })
}