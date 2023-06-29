use crate::error::{ContractError, ContractResult};
use cosmwasm_std::{
    entry_point, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, Uint128, WasmMsg,
};
use cw_utils::one_coin;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    EstimateSwapExactAmountInResponse, EstimateSwapExactAmountOutResponse, MsgSwapExactAmountIn,
    PoolmanagerQuerier, SwapAmountInRoute, SwapAmountOutRoute,
};
use skip::{
    proto_coin::ProtoCoin,
    swap::{
        convert_swap_operations, ExecuteMsg, OsmosisInstantiateMsg as InstantiateMsg, QueryMsg,
        SwapOperation,
    },
};
use std::str::FromStr;

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<Response> {
    Ok(Response::new().add_attribute("action", "instantiate"))
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
        ExecuteMsg::Swap { operations } => execute_swap(env, info, operations),
        ExecuteMsg::TransferFundsBack { caller } => execute_transfer_funds_back(deps, env, caller),
    }
}

// Executes a swap with the given swap operations and then transfers the funds back to the caller
fn execute_swap(
    env: Env,
    info: MessageInfo,
    operations: Vec<SwapOperation>,
) -> ContractResult<Response> {
    // Get coin in from the message info, error if there is not exactly one coin sent
    let coin_in = one_coin(&info)?;

    // Create the osmosis poolmanager swap exact amount in message
    let swap_msg = create_osmosis_swap_msg(&env, coin_in, operations)?;

    // Create the transfer funds back message
    let transfer_funds_back_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::TransferFundsBack {
            caller: info.sender,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(swap_msg)
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_swap_and_transfer_back"))
}

// Query the contract's balance and transfer the funds back to the caller
fn execute_transfer_funds_back(deps: DepsMut, env: Env, caller: Addr) -> ContractResult<Response> {
    // Create the bank message send to transfer the contract funds back to the caller
    let transfer_funds_back_msg = BankMsg::Send {
        to_address: caller.to_string(),
        amount: deps.querier.query_all_balances(env.contract.address)?,
    };

    Ok(Response::new()
        .add_message(transfer_funds_back_msg)
        .add_attribute("action", "dispatch_transfer_funds_back_bank_send"))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// Creates the osmosis poolmanager swap exact amount in message
fn create_osmosis_swap_msg(
    env: &Env,
    coin_in: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<CosmosMsg> {
    // Convert the swap operations to osmosis swap amount in routes
    // Return an error if there was an error converting the swap
    // operations to osmosis swap amount in routes.
    let osmosis_swap_amount_in_routes: Vec<SwapAmountInRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;

    // Create the osmosis poolmanager swap exact amount in message
    // The token out min amount is set to 1 because we are not concerned
    // with the minimum amount in this contract, that gets verified in the
    // entry point contract.
    let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
        sender: env.contract.address.to_string(),
        routes: osmosis_swap_amount_in_routes,
        token_in: Some(ProtoCoin(coin_in).into()),
        token_out_min_amount: "1".to_string(),
    }
    .into();

    Ok(swap_msg)
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
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
        _ => {
            unimplemented!()
        }
    }
    .map_err(From::from)
}

// Queries the osmosis poolmanager module to simulate a swap exact amount in
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

    // Get denom out from last swap operation  to be used as the return coin's denom
    let denom_out = last_op.denom_out.clone();

    // Convert the swap operations to osmosis swap amount in routes
    // Return an error if there was an error converting the swap
    // operations to osmosis swap amount in routes.
    let osmosis_swap_amount_in_routes: Vec<SwapAmountInRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;

    // Query the osmosis poolmanager module to simulate the swap exact amount in
    let res: EstimateSwapExactAmountInResponse = PoolmanagerQuerier::new(&deps.querier)
        .estimate_swap_exact_amount_in(
            osmosis_swap_amount_in_routes.first().unwrap().pool_id,
            coin_in.to_string(),
            osmosis_swap_amount_in_routes,
        )?;

    // Return the coin out
    Ok(Coin {
        denom: denom_out,
        amount: Uint128::from_str(&res.token_out_amount)?,
    })
}

// Queries the osmosis poolmanager module to simulate a swap exact amount out
fn query_simulate_swap_exact_coin_out(
    deps: Deps,
    coin_out: Coin,
    swap_operations: Vec<SwapOperation>,
) -> ContractResult<Coin> {
    // Error if swap operations is empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Ensure coin_out's denom is the same as the last swap operation's denom out
    if coin_out.denom != last_op.denom_out {
        return Err(ContractError::CoinOutDenomMismatch);
    }

    // Get denom in from first swap operation to be used as the return coin's denom
    let denom_in = first_op.denom_in.clone();

    // Convert the swap operations to osmosis swap amount out routes
    // Return an error if there was an error converting the swap
    // operations to osmosis swap amount out routes.
    let osmosis_swap_amount_out_routes: Vec<SwapAmountOutRoute> =
        convert_swap_operations(swap_operations).map_err(ContractError::ParseIntPoolID)?;

    // Query the osmosis poolmanager module to simulate the swap exact amount out
    let res: EstimateSwapExactAmountOutResponse = PoolmanagerQuerier::new(&deps.querier)
        .estimate_swap_exact_amount_out(
            osmosis_swap_amount_out_routes.first().unwrap().pool_id,
            osmosis_swap_amount_out_routes,
            coin_out.to_string(),
        )?;

    // Return the coin in needed
    Ok(Coin {
        denom: denom_in,
        amount: Uint128::from_str(&res.token_in_amount)?,
    })
}
