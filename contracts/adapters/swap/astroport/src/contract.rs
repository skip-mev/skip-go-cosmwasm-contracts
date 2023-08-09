use crate::{
    error::{ContractError, ContractResult},
    state::{ENTRY_POINT_CONTRACT_ADDRESS, ROUTER_CONTRACT_ADDRESS},
};
use astroport::{
    asset::{Asset, AssetInfo},
    pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse},
    router::{
        ExecuteMsg as RouterExecuteMsg, QueryMsg as RouterQueryMsg, SimulateSwapOperationsResponse,
    },
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw_utils::one_coin;
use skip::swap::{
    execute_transfer_funds_back, ExecuteMsg, NeutronInstantiateMsg as InstantiateMsg, QueryMsg,
    SwapOperation,
};

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    // Validate router contract address
    let checked_router_contract_address = deps.api.addr_validate(&msg.router_contract_address)?;

    // Store the router contract address
    ROUTER_CONTRACT_ADDRESS.save(deps.storage, &checked_router_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        )
        .add_attribute(
            "router_contract_address",
            checked_router_contract_address.to_string(),
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
        ExecuteMsg::TransferFundsBack { swapper } => {
            Ok(execute_transfer_funds_back(deps, env, info, swapper)?)
        }
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Get coin in from the message info, error if there is not exactly one coin sent
    let coin_in = one_coin(&info)?;

    // Create the astroport swap message
    let swap_msg = create_astroport_swap_msg(
        ROUTER_CONTRACT_ADDRESS.load(deps.storage)?,
        coin_in,
        operations,
    )?;

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::TransferFundsBack {
            swapper: info.sender,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Converts the swap operations to astroport AstroSwap operations
fn create_astroport_swap_msg(
    router_contract_address: Addr,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<WasmMsg> {
    // Convert the swap operations to astroport swap operations
    let astroport_swap_operations = swap_operations.into_iter().map(From::from).collect();

    // Create the astroport router execute message arguments
    let astroport_router_msg_args = RouterExecuteMsg::ExecuteSwapOperations {
        operations: astroport_swap_operations,
        minimum_receive: None,
        to: None,
        max_spread: None,
    };

    // Create the astroport router swap message
    let swap_msg = WasmMsg::Execute {
        contract_addr: router_contract_address.to_string(),
        msg: to_binary(&astroport_router_msg_args)?,
        funds: vec![coin_in],
    };

    Ok(swap_msg)
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::RouterContractAddress {} => {
            to_binary(&ROUTER_CONTRACT_ADDRESS.load(deps.storage)?)
        }
        QueryMsg::SimulateSwapExactCoinIn {
            coin_in,
            swap_operations,
        } => to_binary(&query_simulate_swap_exact_coin_in(
            deps,
            coin_in,
            swap_operations,
        )?),
        QueryMsg::SimulateSwapExactCoinOut {
            coin_out,
            swap_operations,
        } => to_binary(&query_simulate_swap_exact_coin_out(
            deps,
            coin_out,
            swap_operations,
        )?),
    }
    .map_err(From::from)
}

// Queries the astroport router contract to simulate a swap exact amount in
fn query_simulate_swap_exact_coin_in(
    deps: Deps,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Coin> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure coin_in's denom is the same as the first swap operation's denom in
    if coin_in.denom != first_op.denom_in {
        return Err(ContractError::CoinInDenomMismatch);
    }

    // Get the router contract address
    let router_contract_address = ROUTER_CONTRACT_ADDRESS.load(deps.storage)?;

    // Get denom out from last swap operation
    let denom_out = last_op.denom_out.clone();

    // Convert the swap operations to astroport swap operations
    let astroport_swap_operations = swap_operations.into_iter().map(From::from).collect();

    // Query the astroport router contract to simulate the swap operations
    let res: SimulateSwapOperationsResponse = deps.querier.query_wasm_smart(
        router_contract_address,
        &RouterQueryMsg::SimulateSwapOperations {
            offer_amount: coin_in.amount,
            operations: astroport_swap_operations,
        },
    )?;

    // Return the coin out
    Ok(Coin {
        denom: denom_out,
        amount: res.amount,
    })
}

// Queries the astroport pool contracts to simulate a multi-hop swap exact amount out
fn query_simulate_swap_exact_coin_out(
    deps: Deps,
    coin_out: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Coin> {
    // Error if swap operations is empty
    let Some(last_op) = swap_operations.last() else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    // Iterate through the swap operations in reverse order, querying the astroport pool contracts
    // contracts to get the coin in needed for each swap operation, and then updating the coin in
    // needed for the next swap operation until the coin in needed for the first swap operation is found.
    let coin_in_needed = swap_operations.iter().rev().try_fold(
        coin_out,
        |coin_in_needed, operation| -> Result<_, ContractError> {
            let res: ReverseSimulationResponse = deps.querier.query_wasm_smart(
                &operation.pool,
                &PairQueryMsg::ReverseSimulation {
                    offer_asset_info: None,
                    ask_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: coin_in_needed.denom,
                        },
                        amount: coin_in_needed.amount,
                    },
                },
            )?;

            Ok(Coin {
                denom: operation.denom_in.clone(),
                amount: res.offer_amount.checked_add(Uint128::one())?,
            })
        },
    )?;

    // Return the coin in needed
    Ok(coin_in_needed)
}
