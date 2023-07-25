use crate::{
    error::{ContractError, ContractResult},
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS, SWAP_VENUE_MAP},
};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};
use cw_utils::one_coin;
use skip::{
    coins::Coins,
    entry_point::{Action, Affiliate, ExecuteMsg},
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcInfo, IbcTransfer},
    swap::{ExecuteMsg as SwapExecuteMsg, QueryMsg as SwapQueryMsg, Swap, SwapOperation},
};

///////////////////////////
/// EXECUTE ENTRYPOINTS ///
///////////////////////////

// Main entry point for the contract
// Dispatches the swap and post swap action
#[allow(clippy::too_many_arguments)]
pub fn execute_swap_and_action(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    fee_swap: Option<Swap>,
    user_swap: Swap,
    min_coin: Coin,
    timeout_timestamp: u64,
    post_swap_action: Action,
    refund_action: Option<Action>,
    affiliates: Vec<Affiliate>,
) -> ContractResult<Response> {
    // Create a response object to return
    let mut response: Response = Response::new().add_attribute("action", "execute_swap_and_action");

    // Error if the current block time is greater than the timeout timestamp
    if env.block.time.nanos() > timeout_timestamp {
        return Err(ContractError::Timeout);
    }

    // Get coin sent to the contract from the MessageInfo
    // Error if there is not exactly one coin sent to the contract
    let mut remaining_coin_received = one_coin(&info)?;

    // Add ibc fee messages (swaps and sends) to response if provided
    response = add_fee_msgs_to_response(
        deps,
        response,
        &post_swap_action,
        &refund_action,
        fee_swap,
        &mut remaining_coin_received,
    )?;

    // Create the user swap message
    let user_swap_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::UserSwap {
            user_swap,
            remaining_coin_received,
            min_coin: min_coin.clone(),
            timeout_timestamp,
            refund_action,
        })?,
        funds: vec![],
    };

    // Create the transfer message
    let post_swap_action_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::PostSwapAction {
            min_coin,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        })?,
        funds: vec![],
    };

    // Add the user swap message and post swap action message to the response
    Ok(response
        .add_message(user_swap_msg)
        .add_message(post_swap_action_msg)
        .add_attribute("action", "dispatch_user_swap_and_post_swap_action"))
}

// Dispatches the user swap
// Can only be called by the contract itself
#[allow(clippy::too_many_arguments)]
pub fn execute_user_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_swap: Swap,
    mut remaining_coin_received: Coin,
    min_coin: Coin,
    timeout_timestamp: u64,
    refund_action: Option<Action>,
) -> ContractResult<Response> {
    // Enforce the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Create a response object to return
    let mut response: Response = Response::new().add_attribute("action", "execute_user_swap");

    // Validate swap operations
    validate_swap_operations(
        &user_swap.operations,
        &remaining_coin_received.denom,
        &min_coin.denom,
    )?;

    // Get swap adapter contract address from venue name
    let user_swap_adapter_contract_address =
        SWAP_VENUE_MAP.load(deps.storage, &user_swap.swap_venue_name)?;

    if let Some(refund_action) = refund_action {
        // TODO: Explore just getting the message here and adding to the response
        // Add the refund action message to the response and reduce the remaining_coin_received
        response = add_refund_action_msg_to_response(
            deps,
            response,
            &mut remaining_coin_received,
            refund_action,
            user_swap_adapter_contract_address.clone(),
            min_coin,
            &user_swap.operations,
            timeout_timestamp,
        )?;
    }

    // Create the user swap message args
    let user_swap_msg_args: SwapExecuteMsg = user_swap.into();

    // Create the user swap message
    let user_swap_msg = WasmMsg::Execute {
        contract_addr: user_swap_adapter_contract_address.to_string(),
        msg: to_binary(&user_swap_msg_args)?,
        funds: vec![remaining_coin_received],
    };

    Ok(response
        .add_message(user_swap_msg)
        .add_attribute("action", "dispatch_user_swap"))
}

// Dispatches the post swap action
// Can only be called by the contract itself
pub fn execute_post_swap_action(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_coin: Coin,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
) -> ContractResult<Response> {
    // Enforce the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Create a response object to return
    let mut response: Response =
        Response::new().add_attribute("action", "execute_post_swap_action");

    // Get contract balance of min out coin immediately after swap
    // for fee deduction and transfer out amount enforcement
    let transfer_out_coin_contract_balance_after_swaps = deps
        .querier
        .query_balance(&env.contract.address, &min_coin.denom)?;

    // Error if the contract balance is less than the min out coin amount
    if transfer_out_coin_contract_balance_after_swaps.amount < min_coin.amount {
        return Err(ContractError::ReceivedLessCoinFromSwapsThanMinCoin);
    }

    // Mutable copy of the transfer out coin to subtract fees from
    // to become the final transfer out coin we send to the user
    let mut transfer_out_coin = transfer_out_coin_contract_balance_after_swaps.clone();

    // If affiliates exist, create the affiliate fee messages and add them to the
    // response, decreasing the transfer out coin amount by each affiliate fee amount
    for affiliate in affiliates.iter() {
        // Verify, calculate, and get the affiliate fee amount
        let affiliate_fee_amount = verify_and_calculate_affiliate_fee_amount(
            &deps,
            &transfer_out_coin_contract_balance_after_swaps,
            affiliate,
        )?;

        // Subtract the affiliate fee from the transfer out coin
        transfer_out_coin.amount = transfer_out_coin.amount.checked_sub(affiliate_fee_amount)?;

        // Create the affiliate fee bank send message
        let affiliate_fee_msg = BankMsg::Send {
            to_address: affiliate.address.clone(),
            amount: vec![Coin {
                denom: transfer_out_coin_contract_balance_after_swaps.denom.clone(),
                amount: affiliate_fee_amount,
            }],
        };

        // Add the affiliate fee message and logs to the response
        response = response
            .add_message(affiliate_fee_msg)
            .add_attribute("action", "dispatch_affiliate_fee_bank_send")
            .add_attribute("address", &affiliate.address)
            .add_attribute("amount", affiliate_fee_amount);
    }

    // If affiliates exist, then error if the transfer out coin amount
    // is less than the min coin amount after affiliate fees
    if !affiliates.is_empty() && transfer_out_coin.amount < min_coin.amount {
        return Err(ContractError::TransferOutCoinLessThanMinAfterAffiliateFees);
    }

    // Add the post swap action message to the response
    response = add_action_msg_to_response(
        deps,
        response,
        post_swap_action,
        transfer_out_coin,
        timeout_timestamp,
    )?;

    Ok(response)
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// SWAP MESSAGE HELPER FUNCTIONS

// Creates the fee swap message and returns it
// Also deducts the fee swap in amount from the mutable user swap coin
fn verify_and_create_fee_swap_msg(
    deps: &DepsMut,
    ibc_fees: &Coins,
    fee_swap: Swap,
    remaining_coin_received: &mut Coin,
) -> ContractResult<WasmMsg> {
    // Get coin from ibc fees, errors if a single coin not specified.
    // If no coins specified then a fee swap is not allowed.
    let fee_swap_coin_out = ibc_fees.one_coin()?;

    // Validate swap operations
    validate_swap_operations(
        &fee_swap.operations,
        &remaining_coin_received.denom,
        &fee_swap_coin_out.denom,
    )?;

    // Get swap adapter contract address from venue name
    let fee_swap_adapter_contract_address =
        SWAP_VENUE_MAP.load(deps.storage, &fee_swap.swap_venue_name)?;

    // Get the exact coin_in needed to receive the coin_out
    let fee_swap_coin_in = verify_and_get_exact_coin_in_needed(
        deps,
        fee_swap_adapter_contract_address.clone(),
        fee_swap_coin_out,
        &fee_swap.operations,
        remaining_coin_received.denom.clone(),
    )?;

    // Deduct the fee swap in amount from the swappable coin
    // Error if swap requires more than the swappable coin amount
    remaining_coin_received.amount = remaining_coin_received
        .amount
        .checked_sub(fee_swap_coin_in.amount)?;

    // Create the fee swap message args
    let fee_swap_msg_args: SwapExecuteMsg = fee_swap.into();

    // Create the fee swap message
    let fee_swap_msg = WasmMsg::Execute {
        contract_addr: fee_swap_adapter_contract_address.to_string(),
        msg: to_binary(&fee_swap_msg_args)?,
        funds: vec![fee_swap_coin_in],
    };

    Ok(fee_swap_msg)
}

// Validates the swap operations
fn validate_swap_operations(
    swap_operations: &[SwapOperation],
    coin_in_denom: &str,
    coin_out_denom: &str,
) -> ContractResult<()> {
    // Verify the swap operations are not empty
    let (Some(first_op), Some(last_op)) = (swap_operations.first(), swap_operations.last()) else {
        return Err(ContractError::SwapOperationsEmpty);
    };

    // Verify the first swap operation denom in is the same as the coin in denom
    if first_op.denom_in != coin_in_denom {
        return Err(ContractError::SwapOperationsCoinInDenomMismatch);
    }

    // Verify the last swap operation denom out is the same as the coin out denom
    if last_op.denom_out != coin_out_denom {
        return Err(ContractError::SwapOperationsCoinOutDenomMismatch);
    }

    Ok(())
}

// Validate and retrieve exact coin in needed to obtrain a specified coin out from a set of swap operations
fn verify_and_get_exact_coin_in_needed(
    deps: &DepsMut,
    swap_adapter_contract_address: Addr,
    coin_out: Coin,
    swap_operations: &[SwapOperation],
    denom_in: String,
) -> ContractResult<Coin> {
    // Get the exact coin_in needed to receive the exact_coin_out wanted by the user
    let swap_exact_coin_in: Coin = deps.querier.query_wasm_smart(
        swap_adapter_contract_address,
        &SwapQueryMsg::SimulateSwapExactCoinOut {
            coin_out,
            swap_operations: swap_operations.to_vec(),
        },
    )?;

    // Verify the user swap exact coin in denom is the same as the remaining coin received denom
    if swap_exact_coin_in.denom != denom_in {
        return Err(ContractError::SwapExactCoinInDenomMismatch);
    }

    Ok(swap_exact_coin_in)
}

// IBC FEE HELPER FUNCTIONS

fn add_fee_msgs_to_response(
    deps: DepsMut,
    mut response: Response,
    post_swap_action: &Action,
    refund_action: &Option<Action>,
    fee_swap: Option<Swap>,
    remaining_coin_received: &mut Coin,
) -> ContractResult<Response> {
    // Get the total ibc_fees as a Coins struct
    let ibc_fees = get_total_ibc_fees(post_swap_action, refund_action)?;

    // Process the fee swap if it exists
    if let Some(fee_swap) = fee_swap {
        // Create the fee swap message
        // NOTE: this call mutates the remaining coin received by subtracting the fee swap in amount
        let fee_swap_msg =
            verify_and_create_fee_swap_msg(&deps, &ibc_fees, fee_swap, remaining_coin_received)?;

        // Add the fee swap message to the response
        response = response
            .add_message(fee_swap_msg)
            .add_attribute("action", "dispatch_fee_swap");
    } else {
        // Deduct the amount of the remaining received coin's denomination that matches
        // with the IBC fees from the remaining coin received amount
        remaining_coin_received.amount = remaining_coin_received
            .amount
            .checked_sub(ibc_fees.get_amount(&remaining_coin_received.denom))?;
    }

    // TODO: Potentially abstract to own function
    // If ibc_fees is not empty, add the bank send message that transfers
    // the funds to the ibc transfer adapter conract to the response
    if !ibc_fees.is_empty() {
        // Create a bank send message, sending the fees to the adapter contract
        let ibc_transfer_bank_send_msg = BankMsg::Send {
            to_address: IBC_TRANSFER_CONTRACT_ADDRESS
                .load(deps.storage)?
                .to_string(),
            amount: ibc_fees.into(),
        };

        // Add the bank send message to the response
        response = response
            .add_message(ibc_transfer_bank_send_msg)
            .add_attribute("action", "dispatch_ibc_fee_bank_send_msg");
    }

    Ok(response)
}

// AFFILIATE FEE HELPER FUNCTIONS

// Verifies the affiliate address is valid, if so then
// returns the calculated affiliate fee amount.
fn verify_and_calculate_affiliate_fee_amount(
    deps: &DepsMut,
    transfer_out_coin_contract_balance_after_swaps: &Coin,
    affiliate: &Affiliate,
) -> ContractResult<Uint128> {
    // Verify the affiliate address is valid
    deps.api.addr_validate(&affiliate.address)?;

    // Get the affiliate fee amount by multiplying the transfer out coin amount
    // immediately after the swaps by the affiliate basis points fee divided by 10000
    let affiliate_fee_amount = transfer_out_coin_contract_balance_after_swaps
        .amount
        .multiply_ratio(affiliate.basis_points_fee, Uint128::new(10000));

    Ok(affiliate_fee_amount)
}

// ACTION MESSAGE HELPER FUNCTIONS

// Note: Mutates the remaining_coin_received by subtracting
// the refund amount from it
#[allow(clippy::too_many_arguments)]
fn add_refund_action_msg_to_response(
    deps: DepsMut,
    mut response: Response,
    remaining_coin_received: &mut Coin,
    refund_action: Action,
    user_swap_adapter_contract_address: Addr,
    coin_out: Coin,
    user_swap_operations: &[SwapOperation],
    timeout_timestamp: u64,
) -> ContractResult<Response> {
    // Get the exact coin_in needed to receive the exact_coin_out wanted by the user
    let user_swap_exact_coin_in = verify_and_get_exact_coin_in_needed(
        &deps,
        user_swap_adapter_contract_address,
        coin_out,
        user_swap_operations,
        remaining_coin_received.denom.clone(),
    )?;

    // Take the difference between the remaining coin received and the exact amount in
    // to get the amount to refund to the user.
    let refund_coin: Coin = Coin {
        denom: remaining_coin_received.denom.clone(),
        amount: remaining_coin_received
            .amount
            .checked_sub(user_swap_exact_coin_in.amount)?,
    };

    // Reduce the remaining coin received to match the exact_coin_in
    *remaining_coin_received = user_swap_exact_coin_in;

    // Add the refund action message to the response
    response = add_action_msg_to_response(
        deps,
        response,
        refund_action,
        refund_coin,
        timeout_timestamp,
    )?;

    Ok(response)
}

// Matches the action and adds the appropriate message to the response, erroring if validation is not passed
fn add_action_msg_to_response(
    deps: DepsMut,
    mut response: Response,
    action: Action,
    transfer_out_coin: Coin,
    timeout_timestamp: u64,
) -> ContractResult<Response> {
    match action {
        Action::BankSend { to_address } => {
            // Create the bank send message
            let bank_send_msg =
                verify_and_create_bank_send_msg(deps, to_address, transfer_out_coin)?;

            // Add the bank send message to the response
            response = response
                .add_message(bank_send_msg)
                .add_attribute("action", "dispatch_action_bank_send");
        }
        Action::IbcTransfer { ibc_info } => {
            // Enforce min out w/ ibc fees and create the IBC Transfer adapter contract call message
            let ibc_transfer_adapter_msg = verify_and_create_ibc_transfer_adapter_msg(
                deps,
                timeout_timestamp,
                ibc_info,
                transfer_out_coin,
            )?;

            // Add the IBC transfer message to the response
            response = response
                .add_message(ibc_transfer_adapter_msg)
                .add_attribute("action", "dispatch_action_ibc_transfer");
        }
        Action::ContractCall {
            contract_address,
            msg,
        } => {
            // Verify and create the contract call message
            let contract_call_msg = verify_and_create_contract_call_msg(
                deps,
                contract_address,
                msg,
                transfer_out_coin,
            )?;

            // Add the contract call message to the response
            response = response
                .add_message(contract_call_msg)
                .add_attribute("action", "dispatch_action_contract_call");
        }
    };

    Ok(response)
}

// Do min transfer coin out verification,
// Then create and return a bank send message
fn verify_and_create_bank_send_msg(
    deps: DepsMut,
    to_address: String,
    transfer_out_coin: Coin,
) -> ContractResult<BankMsg> {
    // Error if the destination address is not a valid address on the current chain
    deps.api.addr_validate(&to_address)?;

    // Create the bank send message
    let bank_send_msg = BankMsg::Send {
        to_address,
        amount: vec![transfer_out_coin],
    };

    Ok(bank_send_msg)
}

// Create and return a message that calls the IBC Transfer adapter contract
fn verify_and_create_ibc_transfer_adapter_msg(
    deps: DepsMut,
    timeout_timestamp: u64,
    ibc_info: IbcInfo,
    transfer_out_coin: Coin,
) -> ContractResult<WasmMsg> {
    // Validates recover address, errors if invalid
    deps.api.addr_validate(&ibc_info.recover_address)?;

    // Create the IBC transfer message
    let ibc_transfer_msg: IbcTransferExecuteMsg = IbcTransfer {
        info: ibc_info,
        coin: transfer_out_coin.clone(),
        timeout_timestamp,
    }
    .into();

    // Get the IBC transfer adapter contract address
    let ibc_transfer_contract_address = IBC_TRANSFER_CONTRACT_ADDRESS.load(deps.storage)?;

    // Send the IBC transfer by calling the IBC transfer contract
    let ibc_msg = WasmMsg::Execute {
        contract_addr: ibc_transfer_contract_address.to_string(),
        msg: to_binary(&ibc_transfer_msg)?,
        funds: vec![transfer_out_coin],
    };

    Ok(ibc_msg)
}

// Verifies, creates, and returns the contract call message
fn verify_and_create_contract_call_msg(
    deps: DepsMut,
    contract_address: String,
    msg: Binary,
    transfer_out_coin: Coin,
) -> ContractResult<WasmMsg> {
    // Verify the contract address is valid, error if invalid
    let checked_contract_address = deps.api.addr_validate(&contract_address)?;

    // Error if the contract address is in the blocked contract addresses map
    if BLOCKED_CONTRACT_ADDRESSES.has(deps.storage, &checked_contract_address) {
        return Err(ContractError::ContractCallAddressBlocked);
    }

    // Error if the contract address is in the blocked contract addresses map
    if BLOCKED_CONTRACT_ADDRESSES.has(deps.storage, &checked_contract_address) {
        return Err(ContractError::ContractCallAddressBlocked);
    }

    // Create the contract call message
    let contract_call_msg = WasmMsg::Execute {
        contract_addr: contract_address,
        msg,
        funds: vec![transfer_out_coin],
    };

    Ok(contract_call_msg)
}

// IBC Fee Helpers

// Get the total ibc fees as a Coins struct
fn get_total_ibc_fees(
    post_swap_action: &Action,
    refund_action: &Option<Action>,
) -> ContractResult<Coins> {
    // Get the ibc_info from the post swap action if the post swap action
    // is an IBC transfer, otherwise set it to None
    let mut ibc_fees = match post_swap_action {
        Action::IbcTransfer { ibc_info } => ibc_info.fee.clone().try_into()?,
        _ => Coins::new(),
    };

    // Update the ibc_fees if the refund action is an IBC transfer
    ibc_fees = match refund_action {
        Some(Action::IbcTransfer { ibc_info }) => {
            // Add the refund action IBC fee to the ibc_fees
            ibc_fees.add_ibc_fee(&ibc_info.fee)?;

            // Return the updated ibc_fees
            ibc_fees
        }
        _ => ibc_fees,
    };

    Ok(ibc_fees)
}
