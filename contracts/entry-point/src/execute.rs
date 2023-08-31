use crate::{
    error::{ContractError, ContractResult},
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS, SWAP_VENUE_MAP},
};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use cw_utils::one_coin;
use skip::{
    asset::Asset,
    entry_point::{Action, Affiliate, Cw20HookMsg, ExecuteMsg},
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcTransfer},
    swap::{
        validate_swap_operations, ExecuteMsg as SwapExecuteMsg, QueryMsg as SwapQueryMsg, Swap,
        SwapExactCoinOut,
    },
};

//////////////////////////
/// RECEIVE ENTRYPOINT ///
//////////////////////////

// Receive is the main entry point for the contract to
// receive cw20 tokens and execute the swap and action message
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> ContractResult<Response> {
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::SwapAndAction {
            sent_asset,
            user_swap,
            min_coin, // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
            timeout_timestamp,
            post_swap_action,
            affiliates,
        } => execute_swap_and_action(
            deps,
            env,
            info,
            sent_asset,
            user_swap,
            min_coin,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        ),
    }
}

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
    _sent_asset: Asset,
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
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    let mut remaining_coin = one_coin(&info)?;

    // If the post swap action is an IBC transfer, then handle the ibc fees
    // by either creating a fee swap message or deducting the ibc fees from
    // the remaining coin received amount.
    if let Action::IbcTransfer { ibc_info, fee_swap } = &post_swap_action {
        let ibc_fee_coin = ibc_info
            .fee
            .as_ref()
            .map(|fee| fee.one_coin())
            .transpose()?;

        if let Some(fee_swap) = fee_swap {
            let ibc_fee_coin = ibc_fee_coin
                .clone()
                .ok_or(ContractError::FeeSwapWithoutIbcFees)?;

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
        } else if let Some(ibc_fee_coin) = &ibc_fee_coin {
            if remaining_coin.denom != ibc_fee_coin.denom {
                return Err(ContractError::IBCFeeDenomDiffersFromCoinReceived);
            }

            // Deduct the ibc_fee_coin amount from the remaining coin received amount
            remaining_coin.amount = remaining_coin.amount.checked_sub(ibc_fee_coin.amount)?;
        }

        // Dispatch the ibc fee bank send to the ibc transfer adapter contract if needed
        // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
        if let Some(ibc_fee_coin) = ibc_fee_coin {
            // Get the ibc transfer adapter contract address
            let ibc_transfer_contract_address = IBC_TRANSFER_CONTRACT_ADDRESS.load(deps.storage)?;

            // Create the ibc fee bank send message
            let ibc_fee_msg = BankMsg::Send {
                to_address: ibc_transfer_contract_address.to_string(),
                amount: vec![ibc_fee_coin],
            };

            // Add the ibc fee message to the response
            response = response
                .add_message(ibc_fee_msg)
                .add_attribute("action", "dispatch_ibc_fee_bank_send");
        }
    }

    // Set a boolean to determine if the user swap is exact out or not
    let exact_out = match &user_swap {
        Swap::SwapExactCoinIn(_) => false,
        Swap::SwapExactCoinOut(_) => true,
    };

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
            exact_out,
        })?,
        funds: vec![],
    };

    // Add the post swap action message to the response and return the response
    Ok(response
        .add_message(post_swap_action_msg)
        .add_attribute("action", "dispatch_post_swap_action"))
}

// Dispatches the user swap and refund/affiliate fee bank sends if needed
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

    // Create affiliate response and total affiliate fee amount
    let mut affiliate_response: Response = Response::new();
    let mut total_affiliate_fee_amount: Uint128 = Uint128::zero();

    // If affiliates exist, create the affiliate fee messages and attributes and
    // add them to the affiliate response, updating the total affiliate fee amount
    for affiliate in affiliates.iter() {
        // Verify, calculate, and get the affiliate fee amount
        // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
        let affiliate_fee_amount =
            verify_and_calculate_affiliate_fee_amount(&deps, &min_coin, affiliate)?;

        // Add the affiliate fee amount to the total affiliate fee amount
        // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
        total_affiliate_fee_amount =
            total_affiliate_fee_amount.checked_add(affiliate_fee_amount)?;

        // Create the affiliate fee bank send message
        // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
        let affiliate_fee_msg = BankMsg::Send {
            to_address: affiliate.address.clone(),
            amount: vec![Coin {
                denom: min_coin.denom.clone(),
                amount: affiliate_fee_amount,
            }],
        };

        // Add the affiliate fee message and attributes to the response
        affiliate_response = affiliate_response
            .add_message(affiliate_fee_msg)
            .add_attribute("action", "dispatch_affiliate_fee_bank_send")
            .add_attribute("address", &affiliate.address)
            .add_attribute("amount", affiliate_fee_amount);
    }

    // Create the user swap message
    match swap {
        Swap::SwapExactCoinIn(swap) => {
            // Validate swap operations
            validate_swap_operations(&swap.operations, &remaining_coin.denom, &min_coin.denom)?;

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

            response = response
                .add_message(user_swap_msg)
                .add_attribute("action", "dispatch_user_swap_exact_coin_in");
        }
        Swap::SwapExactCoinOut(swap) => {
            // Validate swap operations
            validate_swap_operations(&swap.operations, &remaining_coin.denom, &min_coin.denom)?;

            // Get swap adapter contract address from venue name
            let user_swap_adapter_contract_address =
                SWAP_VENUE_MAP.load(deps.storage, &swap.swap_venue_name)?;

            // Calculate the swap coin out by adding the min coin amount to the total affiliate fee amount
            // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
            let swap_coin_out = Coin {
                denom: min_coin.denom,
                amount: min_coin.amount.checked_add(total_affiliate_fee_amount)?,
            };

            // Query the swap adapter to get the coin in needed to obtain the min coin plus affiliates
            // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
            let user_swap_coin_in = query_swap_coin_in(
                &deps,
                &user_swap_adapter_contract_address,
                &swap,
                &swap_coin_out,
            )?;

            // Verify the user swap in denom is the same as the denom received from the message to the contract
            // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
            if user_swap_coin_in.denom != remaining_coin.denom {
                return Err(ContractError::UserSwapCoinInDenomMismatch);
            }

            // Calculate refund amount to send back to the user
            // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
            let refund_amount = remaining_coin
                .amount
                .checked_sub(user_swap_coin_in.amount)?;

            // If refund amount gt zero, then create the refund message and add it to the response
            if refund_amount > Uint128::zero() {
                // Get the refund address from the swap
                let to_address = swap
                    .refund_address
                    .clone()
                    .ok_or(ContractError::NoRefundAddress)?;

                // Validate the refund address
                deps.api.addr_validate(&to_address)?;

                // Create the refund message
                // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
                let refund_msg = BankMsg::Send {
                    to_address: to_address.clone(),
                    amount: vec![Coin {
                        denom: remaining_coin.denom,
                        amount: refund_amount,
                    }],
                };

                // Add the refund message and attributes to the response
                response = response
                    .add_message(refund_msg)
                    .add_attribute("action", "dispatch_refund")
                    .add_attribute("address", &to_address)
                    .add_attribute("amount", refund_amount);
            }

            // Create the user swap message args
            let user_swap_msg_args: SwapExecuteMsg = swap.into();

            // Create the user swap message
            let user_swap_msg = WasmMsg::Execute {
                contract_addr: user_swap_adapter_contract_address.to_string(),
                msg: to_binary(&user_swap_msg_args)?,
                funds: vec![user_swap_coin_in],
            };

            response = response
                .add_message(user_swap_msg)
                .add_attribute("action", "dispatch_user_swap_exact_coin_out");
        }
    }

    // Add the affiliate messages and attributes to the response and return the response
    // Having the affiliate messages after the swap is purposeful, so that the affiliate
    // bank sends are valid and the contract has funds to send to the affiliates.
    Ok(response
        .add_submessages(affiliate_response.messages)
        .add_attributes(affiliate_response.attributes))
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
    exact_out: bool,
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
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    let transfer_out_coin = deps
        .querier
        .query_balance(&env.contract.address, &min_coin.denom)?;

    // Error if the contract balance is less than the min out coin amount
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    if transfer_out_coin.amount < min_coin.amount {
        return Err(ContractError::ReceivedLessCoinFromSwapsThanMinCoin);
    }

    // Set the transfer out coin to the min coin if exact out is true
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    let transfer_out_coin = if exact_out {
        min_coin
    } else {
        transfer_out_coin
    };

    match post_swap_action {
        Action::BankSend { to_address } => {
            // Error if the destination address is not a valid address on the current chain
            deps.api.addr_validate(&to_address)?;

            // Create the bank send message
            // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
            let bank_send_msg = BankMsg::Send {
                to_address,
                amount: vec![transfer_out_coin],
            };

            // Add the bank send message to the response
            response = response
                .add_message(bank_send_msg)
                .add_attribute("action", "dispatch_post_swap_bank_send");
        }
        Action::IbcTransfer { ibc_info, .. } => {
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
            let ibc_transfer_msg = WasmMsg::Execute {
                contract_addr: ibc_transfer_contract_address.to_string(),
                msg: to_binary(&ibc_transfer_msg)?,
                funds: vec![transfer_out_coin],
            };

            // Add the IBC transfer message to the response
            response = response
                .add_message(ibc_transfer_msg)
                .add_attribute("action", "dispatch_post_swap_ibc_transfer");
        }
        Action::ContractCall {
            contract_address,
            msg,
        } => {
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
    fee_swap: &SwapExactCoinOut,
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
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    let fee_swap_coin_in = query_swap_coin_in(
        deps,
        &fee_swap_adapter_contract_address,
        fee_swap,
        ibc_fee_coin,
    )?;

    // Verify the fee swap in denom is the same as the denom received from the message to the contract
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    if fee_swap_coin_in.denom != remaining_coin.denom {
        return Err(ContractError::FeeSwapCoinInDenomMismatch);
    }

    // Deduct the fee swap in amount from the swappable coin
    // Error if swap requires more than the swappable coin amount
    // @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
    remaining_coin.amount = remaining_coin.amount.checked_sub(fee_swap_coin_in.amount)?;

    // Create the fee swap message args
    let fee_swap_msg_args: SwapExecuteMsg = fee_swap.clone().into();

    // Create the fee swap message
    let fee_swap_msg = WasmMsg::Execute {
        contract_addr: fee_swap_adapter_contract_address.to_string(),
        msg: to_binary(&fee_swap_msg_args)?,
        funds: vec![fee_swap_coin_in],
    };

    Ok(fee_swap_msg)
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

// QUERY HELPER FUNCTIONS

// Unexposed query helper function that queries the swap adapter contract to get the
// coin in needed for the fee swap. Verifies the fee swap in denom is the same as the
// swap coin denom from the message. Returns the fee swap coin in.
// @NotJeremyLiu TODO: Use Asset instead of Coin For cw-20 support
fn query_swap_coin_in(
    deps: &DepsMut,
    swap_adapter_contract_address: &Addr,
    swap: &SwapExactCoinOut,
    swap_coin_out: &Coin,
) -> ContractResult<Coin> {
    // Query the swap adapter to get the coin in needed for the fee swap
    let fee_swap_coin_in: Coin = deps.querier.query_wasm_smart(
        swap_adapter_contract_address,
        &SwapQueryMsg::SimulateSwapExactCoinOut {
            coin_out: swap_coin_out.clone(),
            swap_operations: swap.operations.clone(),
        },
    )?;

    Ok(fee_swap_coin_in)
}
