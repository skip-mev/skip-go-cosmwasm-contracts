use crate::{
    error::{ContractError, ContractResult},
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS, SWAP_VENUE_MAP},
};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Coins, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw_utils::one_coin;
use skip::{
    entry_point::{Action, Affiliate, ExecuteMsg},
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcInfo, IbcTransfer},
    swap::{
        validate_swap_operations, ExecuteMsg as SwapExecuteMsg, QueryMsg as SwapQueryMsg, Swap,
        SwapExactCoinOut,
    },
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
    fee_swap: Option<SwapExactCoinOut>,
    user_swap: Swap,
    min_coin: Coin,
    timeout_timestamp: u64,
    post_swap_action: Action,
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
    let mut remaining_coin = one_coin(&info)?;

    // If the post swap action is an IBC transfer, then handle the ibc fees
    // by either creating a fee swap message or deducting the ibc fees from
    // the remaining coin received amount. If the post swap action is not an
    // IBC transfer, then error if a fee swap is provided.
    match &post_swap_action {
        Action::IbcTransfer { ibc_info } => {
            let ibc_fee_coin = ibc_info
                .fee
                .as_ref()
                .map(|fee| fee.one_coin())
                .transpose()?;

            if let Some(fee_swap) = fee_swap {
                let ibc_fee_coin = ibc_fee_coin.ok_or(ContractError::FeeSwapWithoutIbcFees)?;

                // NOTE: this call mutates remaining_coin_received by deducting ibc_fee_coin's amount from it
                let fee_swap_msg = verify_and_create_fee_swap_msg(
                    &deps,
                    fee_swap,
                    &mut remaining_coin,
                    &ibc_fee_coin,
                )?;

                // Add the fee swap message to the response
                response = response
                    .add_message(fee_swap_msg)
                    .add_attribute("action", "dispatch_fee_swap");
            } else if let Some(ibc_fee_coin) = ibc_fee_coin {
                if remaining_coin.denom != ibc_fee_coin.denom {
                    return Err(ContractError::IBCFeeDenomDiffersFromCoinReceived);
                }

                // Deduct the ibc_fee_coin amount from the remaining coin received amount
                remaining_coin.amount = remaining_coin.amount.checked_sub(ibc_fee_coin.amount)?;
            }
        }
        _ => {
            if fee_swap.is_some() {
                return Err(ContractError::FeeSwapWithoutIbcTransfer);
            }
        }
    }

    // Create the user swap message
    let user_swap_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::UserSwap {
            swap: user_swap,
            min_coin: min_coin.clone(),
            remaining_coin,
            affiliates,
        })?,
        funds: vec![],
    };

    // Add the user swap message to the response
    response = response
        .add_message(user_swap_msg)
        .add_attribute("action", "dispatch_user_swap");

    // Create the post swap action message
    let post_swap_action_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::PostSwapAction {
            min_coin,
            timeout_timestamp,
            post_swap_action,
        })?,
        funds: vec![],
    };

    // Add the post swap action message to the response and return the response
    Ok(response
        .add_message(post_swap_action_msg)
        .add_attribute("action", "dispatch_post_swap_action"))
}

pub fn execute_user_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap: Swap,
    min_coin: Coin,
    remaining_coin: Coin,
    affiliates: Vec<Affiliate>,
) -> ContractResult<Response> {
    // Enforce the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Create a response object to return
    let mut response: Response = Response::new().add_attribute("action", "execute_user_swap");

    // Create the user swap message
    let user_swap_msg =
        verify_and_create_user_swap_msg(&deps, swap, remaining_coin, &min_coin.denom)?;

    // Add the user swap message to the response
    response = response
        .add_message(user_swap_msg)
        .add_attribute("action", "dispatch_user_swap");

    // If affiliates exist, create the affiliate fee messages and add them to the
    // response, decreasing the transfer out coin amount by each affiliate fee amount
    for affiliate in affiliates.iter() {
        // Verify, calculate, and get the affiliate fee amount
        let affiliate_fee_amount =
            verify_and_calculate_affiliate_fee_amount(&deps, &min_coin, affiliate)?;

        // Create the affiliate fee bank send message
        let affiliate_fee_msg = BankMsg::Send {
            to_address: affiliate.address.clone(),
            amount: vec![Coin {
                denom: min_coin.denom.clone(),
                amount: affiliate_fee_amount,
            }],
        };

        // Add the affiliate fee message and attributes to the response
        response = response
            .add_message(affiliate_fee_msg)
            .add_attribute("action", "dispatch_affiliate_fee_bank_send")
            .add_attribute("address", &affiliate.address)
            .add_attribute("amount", affiliate_fee_amount);
    }

    Ok(response)
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
    let transfer_out_coin = deps
        .querier
        .query_balance(&env.contract.address, &min_coin.denom)?;

    // Error if the contract balance is less than the min out coin amount
    if transfer_out_coin.amount < min_coin.amount {
        return Err(ContractError::ReceivedLessCoinFromSwapsThanMinCoin);
    }

    match post_swap_action {
        Action::BankSend { to_address } => {
            // Create the bank send message
            let bank_send_msg =
                verify_and_create_bank_send_msg(deps, to_address, transfer_out_coin)?;

            // Add the bank send message to the response
            response = response
                .add_message(bank_send_msg)
                .add_attribute("action", "dispatch_post_swap_bank_send");
        }
        Action::IbcTransfer { ibc_info } => {
            // Enforce min out w/ ibc fees and create the IBC Transfer adapter contract call message
            let ibc_transfer_adapter_msg = verify_and_create_ibc_transfer_adapter_msg(
                deps,
                min_coin,
                timeout_timestamp,
                ibc_info,
                transfer_out_coin,
            )?;

            // Add the IBC transfer message to the response
            response = response
                .add_message(ibc_transfer_adapter_msg)
                .add_attribute("action", "dispatch_post_swap_ibc_transfer");
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
                .add_attribute("action", "dispatch_post_swap_contract_call");
        }
    };

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
    fee_swap: SwapExactCoinOut,
    remaining_coin: &mut Coin,
    ibc_fee_coin: &Coin,
) -> ContractResult<WasmMsg> {
    // Validate swap operations
    validate_swap_operations(
        &fee_swap.operations,
        &remaining_coin.denom,
        &ibc_fee_coin.denom,
    )?;

    // Get swap adapter contract address from venue name
    let fee_swap_adapter_contract_address =
        SWAP_VENUE_MAP.load(deps.storage, &fee_swap.swap_venue_name)?;

    // Query the swap adapter to get the coin in needed for the fee swap
    let fee_swap_coin_in = query_swap_coin_in(
        deps,
        &fee_swap_adapter_contract_address,
        &fee_swap,
        ibc_fee_coin,
    )?;

    // Verify the fee swap in denom is the same as the denom received from the message to the contract
    if fee_swap_coin_in.denom != remaining_coin.denom {
        return Err(ContractError::FeeSwapCoinInDenomMismatch);
    }

    // Deduct the fee swap in amount from the swappable coin
    // Error if swap requires more than the swappable coin amount
    remaining_coin.amount = remaining_coin.amount.checked_sub(fee_swap_coin_in.amount)?;

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

// Verifies, creates, and returns the user swap message
fn verify_and_create_user_swap_msg(
    deps: &DepsMut,
    swap: Swap,
    remaining_coin: Coin,
    min_coin_denom: &str,
) -> ContractResult<WasmMsg> {
    match swap {
        Swap::SwapExactCoinIn(swap) => {
            // Validate swap operations
            validate_swap_operations(&swap.operations, &remaining_coin.denom, min_coin_denom)?;

            // Get swap adapter contract address from venue name
            let user_swap_adapter_contract_address =
                SWAP_VENUE_MAP.load(deps.storage, &swap.swap_venue_name)?;

            // Create the user swap message args
            let user_swap_msg_args: SwapExecuteMsg = swap.into();

            // Create the user swap message
            let user_swap_msg = WasmMsg::Execute {
                contract_addr: user_swap_adapter_contract_address.to_string(),
                msg: to_binary(&user_swap_msg_args)?,
                funds: vec![remaining_coin],
            };

            Ok(user_swap_msg)
        }
        Swap::SwapExactCoinOut(_) => {
            unimplemented!()
        }
    }
}

// AFFILIATE FEE HELPER FUNCTIONS

// Verifies the affiliate address is valid, if so then
// returns the calculated affiliate fee amount.
fn verify_and_calculate_affiliate_fee_amount(
    deps: &DepsMut,
    min_coin: &Coin,
    affiliate: &Affiliate,
) -> ContractResult<Uint128> {
    // Verify the affiliate address is valid
    deps.api.addr_validate(&affiliate.address)?;

    // Get the affiliate fee amount by multiplying the min_coin
    // amount by the affiliate basis points fee divided by 10000
    let affiliate_fee_amount = min_coin
        .amount
        .multiply_ratio(affiliate.basis_points_fee, Uint128::new(10000));

    Ok(affiliate_fee_amount)
}

// POST SWAP ACTION MESSAGE HELPER FUNCTIONS

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

// Do min transfer coin out and ibc fee verification,
// Then create and return a message that calls the IBC Transfer adapter contract
fn verify_and_create_ibc_transfer_adapter_msg(
    deps: DepsMut,
    min_coin: Coin,
    timeout_timestamp: u64,
    ibc_info: IbcInfo,
    mut transfer_out_coin: Coin,
) -> ContractResult<WasmMsg> {
    // Validates recover address, errors if invalid
    deps.api.addr_validate(&ibc_info.recover_address)?;

    // Create the ibc_fees map from the given recv_fee, ack_fee, and timeout_fee
    let ibc_fees_map: Coins = ibc_info.fee.clone().unwrap_or_default().try_into()?;

    // Get the amount of the IBC fee payment that matches
    // the denom of the ibc transfer out coin.
    // If there is no denom match, then default to zero.
    let transfer_out_coin_ibc_fee_amount = ibc_fees_map.amount_of(&min_coin.denom);

    // Subtract the IBC fee amount from the transfer out coin
    transfer_out_coin.amount = transfer_out_coin
        .amount
        .checked_sub(transfer_out_coin_ibc_fee_amount)?;

    // Check if the swap out amount after IBC fee is greater than the minimum amount out
    // If it is, then send the IBC transfer, otherwise, return an error
    if transfer_out_coin.amount < min_coin.amount {
        return Err(ContractError::TransferOutCoinLessThanMinAfterIbcFees);
    }

    // Calculate the funds to send to the IBC transfer contract
    // (which is the transfer out coin plus the IBC fee amounts)
    // using a map for convenience, and then converting to a vector of coins
    let mut ibc_msg_funds_map = ibc_fees_map;
    ibc_msg_funds_map.add(transfer_out_coin.clone())?;

    // Convert the map to a vector of coins
    let ibc_msg_funds: Vec<Coin> = ibc_msg_funds_map.into();

    // Create the IBC transfer message
    let ibc_transfer_msg: IbcTransferExecuteMsg = IbcTransfer {
        info: ibc_info,
        coin: transfer_out_coin,
        timeout_timestamp,
    }
    .into();

    // Get the IBC transfer adapter contract address
    let ibc_transfer_contract_address = IBC_TRANSFER_CONTRACT_ADDRESS.load(deps.storage)?;

    // Send the IBC transfer by calling the IBC transfer contract
    let ibc_msg = WasmMsg::Execute {
        contract_addr: ibc_transfer_contract_address.to_string(),
        msg: to_binary(&ibc_transfer_msg)?,
        funds: ibc_msg_funds,
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

    // Create the contract call message
    let contract_call_msg = WasmMsg::Execute {
        contract_addr: contract_address,
        msg,
        funds: vec![transfer_out_coin],
    };

    Ok(contract_call_msg)
}

// QUERY HELPER FUNCTIONS

// Unexposed query helper function that queries the swap adapter contract to get the
// coin in needed for the fee swap. Verifies the fee swap in denom is the same as the
// swap coin denom from the message. Returns the fee swap coin in.
fn query_swap_coin_in(
    deps: &DepsMut,
    swap_adapter_contract_address: &Addr,
    fee_swap: &SwapExactCoinOut,
    fee_swap_coin_out: &Coin,
) -> ContractResult<Coin> {
    // Query the swap adapter to get the coin in needed for the fee swap
    let fee_swap_coin_in: Coin = deps.querier.query_wasm_smart(
        swap_adapter_contract_address,
        &SwapQueryMsg::SimulateSwapExactCoinOut {
            coin_out: fee_swap_coin_out.clone(),
            swap_operations: fee_swap.operations.clone(),
        },
    )?;

    Ok(fee_swap_coin_in)
}
