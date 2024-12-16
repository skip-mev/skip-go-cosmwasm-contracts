use std::vec;

use crate::{
    error::{ContractError, ContractResult},
    hyperlane::{ExecuteMsg as HplExecuteMsg, ExecuteMsg::HplTransfer},
    msg::{Action, Affiliate, ExecuteMsg, Snip20HookMsg, Snip20ReceiveMsg},
    reply::{RecoverTempStorage, RECOVER_REPLY_ID},
    state::{
        BLOCKED_CONTRACT_ADDRESSES, HYPERLANE_TRANSFER_CONTRACT_ADDRESS,
        IBC_TRANSFER_CONTRACT_ADDRESS, PRE_SWAP_OUT_ASSET_AMOUNT, RECOVER_TEMP_STORAGE,
        REGISTERED_TOKENS, SWAP_VENUE_MAP, VIEWING_KEY,
    },
};

use secret_skip::asset::Asset;

use secret_toolkit::snip20;

use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Coin, ContractInfo, CosmosMsg, DepsMut, Env,
    MessageInfo, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20Coin;
use secret_skip::{
    error::SkipError,
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcInfo, IbcTransfer},
    swap::{validate_swap_operations, Swap, SwapExactAssetOut},
};
use skip_go_swap_adapter_shade_protocol::msg::{
    Cw20HookMsg as SwapHookMsg, ExecuteMsg as SwapExecuteMsg, QueryMsg as SwapQueryMsg,
};

//////////////////////////
/// RECEIVE ENTRYPOINT ///
//////////////////////////

// Receive is the main entry point for the contract to
// receive snip20 tokens and execute the swap and action message
pub fn receive_snip20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    snip20_msg: Snip20ReceiveMsg,
) -> ContractResult<Response> {
    let sent_asset = Asset::Cw20(Cw20Coin {
        address: info.sender.to_string(),
        amount: snip20_msg.amount.u128().into(),
    });

    let msg = match snip20_msg.msg {
        Some(msg) => msg,
        None => {
            return Err(ContractError::NoSnip20ReceiveMsg);
        }
    };
    match from_binary(&msg)? {
        Snip20HookMsg::SwapAndActionWithRecover {
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
            recovery_addr,
        } => execute_swap_and_action_with_recover(
            deps,
            env,
            info,
            Some(sent_asset),
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
            recovery_addr,
        ),
        Snip20HookMsg::SwapAndAction {
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        } => execute_swap_and_action(
            deps,
            env,
            info,
            Some(sent_asset),
            user_swap,
            min_asset,
            timeout_timestamp,
            post_swap_action,
            affiliates,
        ),
        Snip20HookMsg::Action {
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
        } => execute_action(
            deps,
            env,
            info,
            Some(sent_asset),
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
        ),
        Snip20HookMsg::ActionWithRecover {
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
            recovery_addr,
        } => execute_action_with_recover(
            deps,
            env,
            info,
            Some(sent_asset),
            timeout_timestamp,
            action,
            exact_out,
            min_asset,
            recovery_addr,
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
    sent_asset: Option<Asset>,
    mut user_swap: Swap,
    min_asset: Asset,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
) -> ContractResult<Response> {
    // Create a response object to return
    let mut response: Response = Response::new().add_attribute("action", "execute_swap_and_action");

    // Validate and unwrap the sent asset
    let sent_asset = match sent_asset {
        Some(sent_asset) => {
            match &sent_asset {
                Asset::Cw20(cw20) => {
                    if cw20.address != info.sender.to_string() {
                        return Err(ContractError::InvalidSnip20Sender);
                    }
                }
                Asset::Native(_) => {
                    return Err(ContractError::NativeCoinNotSupported);
                }
            }
            // sent_asset.validate(&deps, &env, &info)?;
            sent_asset
        }
        None => {
            return Err(ContractError::NativeCoinNotSupported);
        }
    };

    // Error if the current block time is greater than the timeout timestamp
    if env.block.time.nanos() > timeout_timestamp {
        return Err(ContractError::Timeout);
    }

    let viewing_key = VIEWING_KEY.load(deps.storage)?;
    let min_asset_contract =
        REGISTERED_TOKENS.load(deps.storage, deps.api.addr_validate(min_asset.denom())?)?;

    // Save the current out asset amount to storage as the pre swap out asset amount
    let pre_swap_out_asset_amount = match snip20::balance_query(
        deps.querier,
        env.contract.address.to_string(),
        viewing_key,
        255,
        min_asset_contract.code_hash.clone(),
        min_asset_contract.address.to_string(),
    ) {
        Ok(balance) => balance.amount,
        Err(e) => return Err(ContractError::Std(e)),
    };
    PRE_SWAP_OUT_ASSET_AMOUNT.save(deps.storage, &pre_swap_out_asset_amount)?;

    // Already validated at entrypoints (both direct and snip20_receive)
    let mut remaining_asset = sent_asset;

    // If the post swap action is an IBC transfer, then handle the ibc fees
    // by either creating a fee swap message or deducting the ibc fees from
    // the remaining asset received amount.
    if let Action::IbcTransfer { ibc_info, fee_swap } = &post_swap_action {
        response =
            handle_ibc_transfer_fees(&deps, ibc_info, fee_swap, &mut remaining_asset, response)?;
    }

    // Set a boolean to determine if the user swap is exact out or not
    let exact_out = match &user_swap {
        Swap::SwapExactAssetIn(_) => false,
        Swap::SwapExactAssetOut(_) => true,
        Swap::SmartSwapExactAssetIn(_) => false,
    };

    if let Swap::SmartSwapExactAssetIn(smart_swap) = &mut user_swap {
        if smart_swap.routes.is_empty() {
            return Err(ContractError::Skip(SkipError::RoutesEmpty));
        }

        match smart_swap
            .amount()
            .cmp(&remaining_asset.amount().u128().into())
        {
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Less => {
                let diff = remaining_asset
                    .amount()
                    .checked_sub(smart_swap.amount().u128().into())?;

                // If the total swap in amount is less than remaining asset,
                // adjust the routes to match the remaining asset amount
                let largest_route_idx = smart_swap.largest_route_index()?;

                smart_swap.routes[largest_route_idx]
                    .offer_asset
                    .add(diff.u128().into())?;
            }
            std::cmp::Ordering::Greater => {
                let diff = smart_swap
                    .amount()
                    .checked_sub(remaining_asset.amount().u128().into())?;

                // If the total swap in amount is greater than remaining asset,
                // adjust the routes to match the remaining asset amount
                let largest_route_idx = smart_swap.largest_route_index()?;

                smart_swap.routes[largest_route_idx].offer_asset.sub(diff)?;
            }
        }
    }

    let user_swap_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        code_hash: env.contract.code_hash.clone(),
        msg: to_binary(&ExecuteMsg::UserSwap {
            swap: user_swap,
            min_asset: min_asset.clone(),
            remaining_asset,
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
        code_hash: env.contract.code_hash.clone(),
        msg: to_binary(&ExecuteMsg::PostSwapAction {
            min_asset,
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

// Entrypoint that catches all errors in SwapAndAction and recovers
// the original funds sent to the contract to a recover address.
#[allow(clippy::too_many_arguments)]
pub fn execute_swap_and_action_with_recover(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sent_asset: Option<Asset>,
    user_swap: Swap,
    min_asset: Asset,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
    recovery_addr: Addr,
) -> ContractResult<Response> {
    let mut assets: Vec<Asset> = info.funds.iter().cloned().map(Asset::Native).collect();

    if let Some(asset) = &sent_asset {
        if let Asset::Cw20(_) = asset {
            assets.push(asset.clone());
        }
    }

    // Store all parameters into a temporary storage.
    RECOVER_TEMP_STORAGE.save(
        deps.storage,
        &RecoverTempStorage {
            assets,
            recovery_addr,
        },
    )?;

    // Then call ExecuteMsg::SwapAndAction using a SubMsg.
    let sub_msg = SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            code_hash: env.contract.code_hash,
            msg: to_binary(&ExecuteMsg::SwapAndAction {
                sent_asset,
                user_swap,
                min_asset,
                timeout_timestamp,
                post_swap_action,
                affiliates,
            })?,
            funds: info.funds,
        }),
        RECOVER_REPLY_ID,
    );

    Ok(Response::new().add_submessage(sub_msg))
}

// Dispatches the user swap and refund/affiliate fee bank sends if needed
pub fn execute_user_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap: Swap,
    mut min_asset: Asset,
    mut remaining_asset: Asset,
    affiliates: Vec<Affiliate>,
) -> ContractResult<Response> {
    // Enforce the caller is the contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized);
    }

    // Create a response object to return
    let mut response: Response = Response::new()
        .add_attribute("action", "execute_user_swap")
        .add_attribute("denom_in", remaining_asset.denom())
        .add_attribute("denom_out", min_asset.denom());

    // Create affiliate response and total affiliate fee amount
    let mut affiliate_response: Response = Response::new();
    let mut total_affiliate_fee_amount: Uint128 = Uint128::zero();

    // If affiliates exist, create the affiliate fee messages and attributes and
    // add them to the affiliate response, updating the total affiliate fee amount
    for affiliate in affiliates.iter() {
        // Verify, calculate, and get the affiliate fee amount
        let affiliate_fee_amount =
            verify_and_calculate_affiliate_fee_amount(&deps, &min_asset, affiliate)?;

        if affiliate_fee_amount > Uint128::zero() {
            // Add the affiliate fee amount to the total affiliate fee amount
            total_affiliate_fee_amount =
                total_affiliate_fee_amount.checked_add(affiliate_fee_amount)?;

            // Create the affiliate_fee_asset
            let affiliate_fee_asset = Asset::new(deps.api, min_asset.denom(), affiliate_fee_amount);
            let affiliate_fee_contract = REGISTERED_TOKENS.load(
                deps.storage,
                deps.api.addr_validate(affiliate_fee_asset.denom())?,
            )?;

            // Create the affiliate fee message
            // let affiliate_fee_msg = affiliate_fee_asset.transfer(&affiliate.address);
            let affiliate_fee_msg = match snip20::transfer_msg(
                affiliate.address.to_string(),
                affiliate_fee_asset.amount(),
                None,
                None,
                255,
                affiliate_fee_contract.code_hash.clone(),
                affiliate_fee_contract.address.to_string(),
            ) {
                Ok(msg) => msg,
                Err(e) => return Err(ContractError::Std(e)),
            };

            // Add the affiliate fee message and attributes to the response
            affiliate_response = affiliate_response
                .add_message(affiliate_fee_msg)
                .add_attribute("action", "dispatch_affiliate_fee_bank_send")
                .add_attribute("address", &affiliate.address)
                .add_attribute("amount", affiliate_fee_amount);
        }
    }

    let remaining_asset_contract = REGISTERED_TOKENS.load(
        deps.storage,
        deps.api.addr_validate(remaining_asset.denom())?,
    )?;

    // Create the user swap message
    match swap {
        Swap::SwapExactAssetIn(swap) => {
            // Validate swap operations
            validate_swap_operations(&swap.operations, remaining_asset.denom(), min_asset.denom())?;

            // Get swap adapter contract address from venue name
            let user_swap_adapter_contract =
                SWAP_VENUE_MAP.load(deps.storage, &swap.swap_venue_name)?;

            // Create the user swap message args
            let user_swap_msg_args = SwapHookMsg::Swap {
                operations: swap.operations,
            };

            // Create the user swap message
            /*
            let user_swap_msg = remaining_asset.into_wasm_msg(
                user_swap_adapter_contract_address.to_string(),
                to_binary(&user_swap_msg_args)?,
            )?;
            */

            let user_swap_msg = match snip20::send_msg(
                user_swap_adapter_contract.address.to_string(),
                remaining_asset.amount(),
                Some(to_binary(&user_swap_msg_args)?),
                None,
                None,
                255,
                remaining_asset_contract.code_hash.clone(),
                remaining_asset_contract.address.to_string(),
            ) {
                Ok(msg) => msg,
                Err(e) => return Err(ContractError::Std(e)),
            };

            response = response
                .add_message(user_swap_msg)
                .add_attribute("action", "dispatch_user_swap_exact_asset_in");
        }
        Swap::SwapExactAssetOut(swap) => {
            // Validate swap operations
            validate_swap_operations(&swap.operations, remaining_asset.denom(), min_asset.denom())?;

            // Get swap adapter contract address from venue name
            let user_swap_adapter_contract =
                SWAP_VENUE_MAP.load(deps.storage, &swap.swap_venue_name)?;

            // Calculate the swap asset out by adding the min asset amount to the total affiliate fee amount
            min_asset.add(total_affiliate_fee_amount)?;

            // Query the swap adapter to get the asset in needed to obtain the min asset plus affiliates
            let user_swap_asset_in =
                query_swap_asset_in(&deps, &user_swap_adapter_contract, &swap, &min_asset)?;

            // Verify the user swap in denom is the same as the denom received from the message to the contract
            if user_swap_asset_in.denom() != remaining_asset.denom() {
                return Err(ContractError::UserSwapAssetInDenomMismatch);
            }

            // Calculate refund amount to send back to the user
            remaining_asset.sub(user_swap_asset_in.amount())?;

            // If refund amount gt zero, then create the refund message and add it to the response
            if remaining_asset.amount() > Uint128::zero() {
                // Get the refund address from the swap
                let to_address = swap
                    .refund_address
                    .clone()
                    .ok_or(ContractError::NoRefundAddress)?;

                // Validate the refund address
                deps.api.addr_validate(&to_address)?;

                // Get the refund amount
                let refund_amount = remaining_asset.amount();

                let remaining_asset_contract = REGISTERED_TOKENS.load(
                    deps.storage,
                    deps.api.addr_validate(remaining_asset.denom())?,
                )?;
                // Create the refund message
                // let refund_msg = remaining_asset.transfer(&to_address);
                let refund_msg = match snip20::send_msg(
                    to_address.to_string(),
                    remaining_asset.amount(),
                    None,
                    None,
                    None,
                    255,
                    remaining_asset_contract.code_hash.clone(),
                    remaining_asset_contract.address.to_string(),
                ) {
                    Ok(msg) => msg,
                    Err(e) => return Err(ContractError::Std(e)),
                };

                // Add the refund message and attributes to the response
                response = response
                    .add_message(refund_msg)
                    .add_attribute("action", "dispatch_refund")
                    .add_attribute("address", &to_address)
                    .add_attribute("amount", refund_amount);
            }

            // Create the user swap message args
            let user_swap_msg_args = swap;

            // Create the user swap message
            /*
            let user_swap_msg = user_swap_asset_in.into_wasm_msg(
                user_swap_adapter_contract.address.to_string(),
                to_binary(&user_swap_msg_args)?,
            )?;
            */
            let user_swap_msg = match snip20::send_msg(
                user_swap_adapter_contract.address.to_string(),
                remaining_asset.amount(),
                Some(to_binary(&user_swap_msg_args)?),
                None,
                None,
                255,
                remaining_asset_contract.code_hash.clone(),
                remaining_asset_contract.address.to_string(),
            ) {
                Ok(msg) => msg,
                Err(e) => return Err(ContractError::Std(e)),
            };

            response = response
                .add_message(user_swap_msg)
                .add_attribute("action", "dispatch_user_swap_exact_asset_out");
        }
        Swap::SmartSwapExactAssetIn(swap) => {
            for route in swap.routes {
                // Validate swap operations
                validate_swap_operations(
                    &route.operations,
                    remaining_asset.denom(),
                    min_asset.denom(),
                )?;

                // Get swap adapter contract address from venue name
                let user_swap_adapter_contract =
                    SWAP_VENUE_MAP.load(deps.storage, &swap.swap_venue_name)?;

                // Create the user swap message args
                let user_swap_msg_args = SwapHookMsg::Swap {
                    operations: route.operations,
                };

                // Create the user swap message
                /*
                let user_swap_msg = route.offer_asset.into_wasm_msg(
                    user_swap_adapter_contract_address.to_string(),
                    to_binary(&user_swap_msg_args)?,
                )?;
                */
                let user_swap_msg = match snip20::send_msg(
                    user_swap_adapter_contract.address.to_string(),
                    remaining_asset.amount(),
                    Some(to_binary(&user_swap_msg_args)?),
                    None,
                    None,
                    255,
                    remaining_asset_contract.code_hash.clone(),
                    remaining_asset_contract.address.to_string(),
                ) {
                    Ok(msg) => msg,
                    Err(e) => return Err(ContractError::Std(e)),
                };

                response = response
                    .add_message(user_swap_msg)
                    .add_attribute("action", "dispatch_user_swap_exact_asset_in");
            }
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
    min_asset: Asset,
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

    // Get the pre swap out asset amount from storage
    let pre_swap_out_asset_amount = PRE_SWAP_OUT_ASSET_AMOUNT.load(deps.storage)?;

    // Get contract balance of min out asset post swap
    // for fee deduction and transfer out amount enforcement
    // let post_swap_out_asset = get_current_asset_available(&deps, &env, min_asset.denom())?;
    let min_asset_contract =
        REGISTERED_TOKENS.load(deps.storage, deps.api.addr_validate(min_asset.denom())?)?;
    let viewing_key = VIEWING_KEY.load(deps.storage)?;
    let post_swap_out_asset_amount = match snip20::balance_query(
        deps.querier,
        env.contract.address.to_string(),
        viewing_key,
        255,
        min_asset_contract.code_hash.clone(),
        min_asset_contract.address.to_string(),
    ) {
        Ok(balance) => balance.amount,
        Err(e) => return Err(ContractError::Std(e)),
    };

    // Set the transfer out asset to the post swap out asset amount minus the pre swap out asset amount
    // Since we only want to transfer out the amount received from the swap
    let transfer_out_asset = Asset::new(
        deps.api,
        min_asset.denom(),
        post_swap_out_asset_amount - pre_swap_out_asset_amount,
    );

    // Error if the contract balance is less than the min asset out amount
    if transfer_out_asset.amount() < min_asset.amount() {
        return Err(ContractError::ReceivedLessAssetFromSwapsThanMinAsset);
    }

    // Set the transfer out asset to the min asset if exact out is true
    let transfer_out_asset = if exact_out {
        min_asset
    } else {
        transfer_out_asset
    };

    response = response
        .add_attribute(
            "post_swap_action_amount_out",
            transfer_out_asset.amount().to_string(),
        )
        .add_attribute("post_swap_action_denom_out", transfer_out_asset.denom());

    // Dispatch the action message
    response = validate_and_dispatch_action(
        deps,
        post_swap_action,
        transfer_out_asset,
        timeout_timestamp,
        response,
    )?;

    Ok(response)
}

// Dispatches an action
#[allow(clippy::too_many_arguments)]
pub fn execute_action(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sent_asset: Option<Asset>,
    timeout_timestamp: u64,
    action: Action,
    exact_out: bool,
    min_asset: Option<Asset>,
) -> ContractResult<Response> {
    // Create a response object to return
    let mut response: Response = Response::new().add_attribute("action", "execute_action");

    // Validate and unwrap the sent asset
    let sent_asset = match sent_asset {
        Some(sent_asset) => {
            // sent_asset.validate(&deps, &env, &info)?;
            // TODO validate
            sent_asset
        }
        None => {
            return Err(ContractError::NativeCoinNotSupported);
        }
    };

    // Error if the current block time is greater than the timeout timestamp
    if env.block.time.nanos() > timeout_timestamp {
        return Err(ContractError::Timeout);
    }

    // Already validated at entrypoints (both direct and snip20_receive)
    let mut remaining_asset = sent_asset;

    // If the post swap action is an IBC transfer, then handle the ibc fees
    // by either creating a fee swap message or deducting the ibc fees from
    // the remaining asset received amount.
    if let Action::IbcTransfer { ibc_info, fee_swap } = &action {
        response =
            handle_ibc_transfer_fees(&deps, ibc_info, fee_swap, &mut remaining_asset, response)?;
    }

    // Validate and determine the asset to be used for the action
    let action_asset = if exact_out {
        let min_asset = min_asset.ok_or(ContractError::NoMinAssetProvided)?;

        // Ensure remaining_asset and min_asset have the same denom
        if remaining_asset.denom() != min_asset.denom() {
            return Err(ContractError::ActionDenomMismatch);
        }

        // Ensure remaining_asset is greater than or equal to min_asset
        if remaining_asset.amount() < min_asset.amount() {
            return Err(ContractError::RemainingAssetLessThanMinAsset);
        }

        min_asset
    } else {
        remaining_asset.clone()
    };

    // Dispatch the action message
    response =
        validate_and_dispatch_action(deps, action, action_asset, timeout_timestamp, response)?;

    // Return the response
    Ok(response)
}

// Entrypoint that catches all errors in Action and recovers
// the original funds sent to the contract to a recover address.
#[allow(clippy::too_many_arguments)]
pub fn execute_action_with_recover(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sent_asset: Option<Asset>,
    timeout_timestamp: u64,
    action: Action,
    exact_out: bool,
    min_asset: Option<Asset>,
    recovery_addr: Addr,
) -> ContractResult<Response> {
    let mut assets: Vec<Asset> = info.funds.iter().cloned().map(Asset::Native).collect();

    if let Some(asset) = &sent_asset {
        if let Asset::Cw20(_) = asset {
            assets.push(asset.clone());
        }
    }

    // Store all parameters into a temporary storage.
    RECOVER_TEMP_STORAGE.save(
        deps.storage,
        &RecoverTempStorage {
            assets,
            recovery_addr,
        },
    )?;

    // Then call ExecuteMsg::Action using a SubMsg.
    let sub_msg = SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            code_hash: env.contract.code_hash.to_string(),
            msg: to_binary(&ExecuteMsg::Action {
                sent_asset,
                timeout_timestamp,
                action,
                exact_out,
                min_asset,
            })?,
            funds: info.funds,
        }),
        RECOVER_REPLY_ID,
    );

    Ok(Response::new().add_submessage(sub_msg))
}

////////////////////////
/// HELPER FUNCTIONS ///
////////////////////////

// ACTION HELPER FUNCTIONS

// Validates and adds an action message to the response
fn validate_and_dispatch_action(
    deps: DepsMut,
    action: Action,
    action_asset: Asset,
    timeout_timestamp: u64,
    mut response: Response,
) -> Result<Response, ContractError> {
    match action {
        Action::Transfer { to_address } => {
            // Error if the destination address is not a valid address on the current chain
            deps.api.addr_validate(&to_address)?;

            // Create the transfer message
            // let transfer_msg = action_asset.transfer(&to_address);
            let action_asset_contract = REGISTERED_TOKENS
                .load(deps.storage, deps.api.addr_validate(action_asset.denom())?)?;
            let transfer_msg = match snip20::transfer_msg(
                to_address.to_string(),
                action_asset.amount(),
                None,
                None,
                255,
                action_asset_contract.code_hash.clone(),
                action_asset_contract.address.to_string(),
            ) {
                Ok(msg) => msg,
                Err(e) => return Err(ContractError::Std(e)),
            };

            // Add the transfer message to the response
            response = response
                .add_message(transfer_msg)
                .add_attribute("action", "dispatch_action_transfer");
        }
        Action::IbcTransfer { ibc_info, .. } => {
            // Validates recover address, errors if invalid
            deps.api.addr_validate(&ibc_info.recover_address)?;

            let transfer_out_contract = match action_asset {
                Asset::Native(coin) => {
                    return Err(ContractError::NativeCoinNotSupported);
                }
                _ => REGISTERED_TOKENS
                    .load(deps.storage, deps.api.addr_validate(action_asset.denom())?)?,
            };

            // Create the IBC transfer message
            // TODO send ICS20 message
            /*
            let ibc_transfer_msg: IbcTransferExecuteMsg = IbcTransfer {
                info: ibc_info,
                coin: action_asset.clone(),
                timeout_timestamp,
            }
            .into();
            */

            // Get the IBC transfer adapter contract address
            let ibc_transfer_contract = IBC_TRANSFER_CONTRACT_ADDRESS.load(deps.storage)?;

            // Send the IBC transfer by calling the IBC transfer contract
            /*
            let ibc_transfer_msg = WasmMsg::Execute {
                contract_addr: ibc_transfer_contract.address.to_string(),
                code_hash: ibc_transfer_contract.code_hash.clone(),
                msg: to_binary(&ibc_transfer_msg)?,
                funds: vec![],
            };*/

            let ibc_transfer_msg = match snip20::send_msg(
                ibc_transfer_contract.address.to_string(),
                action_asset.amount(),
                None, //Some(to_binary(&fee_swap_msg_args)?),
                None,
                None,
                255,
                transfer_out_contract.code_hash.clone(),
                transfer_out_contract.address.to_string(),
            ) {
                Ok(msg) => match msg {
                    CosmosMsg::Wasm(wasm_msg) => wasm_msg,
                    _ => return Err(ContractError::Std(StdError::generic_err("Invalid WasmMsg"))),
                },
                Err(e) => return Err(ContractError::Std(e)),
            };

            // Add the IBC transfer message to the response
            response = response
                .add_message(ibc_transfer_msg)
                .add_attribute("action", "dispatch_action_ibc_transfer");
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

            let action_asset_contract = REGISTERED_TOKENS
                .load(deps.storage, deps.api.addr_validate(action_asset.denom())?)?;

            // Create the contract call message
            let contract_call_msg = WasmMsg::Execute {
                contract_addr: action_asset_contract.address.to_string(),
                code_hash: action_asset_contract.code_hash,
                msg: to_binary(&msg)?,
                funds: vec![],
            };

            // Add the contract call message to the response
            response = response
                .add_message(contract_call_msg)
                .add_attribute("action", "dispatch_action_contract_call");
        }
        Action::HplTransfer {
            dest_domain,
            recipient,
            hook,
            metadata,
            warp_address,
        } => {
            let transfer_out_coin = match action_asset {
                Asset::Native(coin) => coin,
                _ => return Err(ContractError::NonNativeHplTransfer),
            };

            // Create the Hyperlane transfer message
            let hpl_transfer_msg: HplExecuteMsg = HplTransfer {
                dest_domain,
                recipient,
                hook,
                metadata,
                warp_address,
            };

            // Get the Hyperlane transfer adapter contract address
            let hpl_transfer_contract = HYPERLANE_TRANSFER_CONTRACT_ADDRESS.load(deps.storage)?;

            // Send the Hyperlane transfer by calling the Hyperlane transfer contract
            let hpl_transfer_msg = WasmMsg::Execute {
                contract_addr: hpl_transfer_contract.address.to_string(),
                code_hash: hpl_transfer_contract.code_hash,
                msg: to_binary(&hpl_transfer_msg)?,
                funds: vec![transfer_out_coin],
            };

            // Add the Hyperlane transfer message to the response
            response = response
                .add_message(hpl_transfer_msg)
                .add_attribute("action", "dispatch_action_ibc_transfer");
        }
    };

    Ok(response)
}

// IBC FEE HELPER FUNCTIONS

// Creates the fee swap and ibc transfer messages and adds them to the response
fn handle_ibc_transfer_fees(
    deps: &DepsMut,
    ibc_info: &IbcInfo,
    fee_swap: &Option<SwapExactAssetOut>,
    remaining_asset: &mut Asset,
    mut response: Response,
) -> Result<Response, ContractError> {
    let ibc_fee_coin = ibc_info
        .fee
        .as_ref()
        .map(|fee| fee.one_coin())
        .transpose()?;

    if let Some(fee_swap) = fee_swap {
        let ibc_fee_coin = ibc_fee_coin
            .clone()
            .ok_or(ContractError::FeeSwapWithoutIbcFees)?;

        // NOTE: this call mutates remaining_asset by deducting ibc_fee_coin's amount from it
        let fee_swap_msg =
            verify_and_create_fee_swap_msg(deps, fee_swap, remaining_asset, &ibc_fee_coin)?;

        // Add the fee swap message to the response
        response = response
            .add_message(fee_swap_msg)
            .add_attribute("action", "dispatch_fee_swap");
    } else if let Some(ibc_fee_coin) = &ibc_fee_coin {
        if remaining_asset.denom() != ibc_fee_coin.denom {
            return Err(ContractError::IBCFeeDenomDiffersFromAssetReceived);
        }

        // Deduct the ibc_fee_coin amount from the remaining asset amount
        remaining_asset.sub(ibc_fee_coin.amount)?;
    }

    // Dispatch the ibc fee bank send to the ibc transfer adapter contract if needed
    if let Some(ibc_fee_coin) = ibc_fee_coin {
        // Get the ibc transfer adapter contract address
        let ibc_transfer_contract = IBC_TRANSFER_CONTRACT_ADDRESS.load(deps.storage)?;

        // Create the ibc fee bank send message
        let ibc_fee_msg = BankMsg::Send {
            to_address: ibc_transfer_contract.address.to_string(),
            amount: vec![ibc_fee_coin],
        };

        // Add the ibc fee message to the response
        response = response
            .add_message(ibc_fee_msg)
            .add_attribute("action", "dispatch_ibc_fee_bank_send");
    }

    Ok(response)
}

// SWAP MESSAGE HELPER FUNCTIONS

// Creates the fee swap message and returns it
// Also deducts the fee swap in amount from the mutable remaining asset
fn verify_and_create_fee_swap_msg(
    deps: &DepsMut,
    fee_swap: &SwapExactAssetOut,
    remaining_asset: &mut Asset,
    ibc_fee_coin: &Coin,
) -> ContractResult<WasmMsg> {
    // Validate swap operations
    validate_swap_operations(
        &fee_swap.operations,
        remaining_asset.denom(),
        &ibc_fee_coin.denom,
    )?;

    // Get swap adapter contract address from venue name
    let fee_swap_adapter_contract = SWAP_VENUE_MAP.load(deps.storage, &fee_swap.swap_venue_name)?;

    // Query the swap adapter to get the asset in needed for the fee swap
    let fee_swap_asset_in = query_swap_asset_in(
        deps,
        &fee_swap_adapter_contract,
        fee_swap,
        &ibc_fee_coin.clone().into(),
    )?;

    // Verify the fee swap in denom is the same as the denom received from the message to the contract
    if fee_swap_asset_in.denom() != remaining_asset.denom() {
        return Err(ContractError::FeeSwapAssetInDenomMismatch);
    }

    // Deduct the fee swap in amount from the remaining asset amount
    // Error if swap requires more than the remaining asset amount
    remaining_asset.sub(fee_swap_asset_in.amount())?;

    // Create the fee swap message args
    let fee_swap_msg_args = fee_swap.clone();

    let fee_swap_asset_contract = REGISTERED_TOKENS.load(
        deps.storage,
        deps.api.addr_validate(fee_swap_asset_in.denom())?,
    )?;

    // Create the fee swap message
    /*
    let fee_swap_msg = fee_swap_asset_in.into_wasm_msg(
        fee_swap_adapter_contract.address.to_string(),
        to_binary(&fee_swap_msg_args)?,
    )?;
    */
    let fee_swap_msg = match snip20::send_msg(
        fee_swap_adapter_contract.address.to_string(),
        remaining_asset.amount(),
        Some(to_binary(&fee_swap_msg_args)?),
        None,
        None,
        255,
        fee_swap_asset_contract.code_hash.clone(),
        fee_swap_asset_contract.address.to_string(),
    ) {
        Ok(msg) => match msg {
            CosmosMsg::Wasm(wasm_msg) => wasm_msg,
            _ => return Err(ContractError::Std(StdError::generic_err("Invalid WasmMsg"))),
        },
        Err(e) => return Err(ContractError::Std(e)),
    };

    Ok(fee_swap_msg)
}

// AFFILIATE FEE HELPER FUNCTIONS

// Verifies the affiliate address is valid, if so then
// returns the calculated affiliate fee amount.
fn verify_and_calculate_affiliate_fee_amount(
    deps: &DepsMut,
    min_asset: &Asset,
    affiliate: &Affiliate,
) -> ContractResult<Uint128> {
    // Verify the affiliate address is valid
    deps.api.addr_validate(&affiliate.address)?;

    // Get the affiliate fee amount by multiplying the min_asset
    // amount by the affiliate basis points fee divided by 10000
    let affiliate_fee_amount = min_asset
        .amount()
        .multiply_ratio(affiliate.basis_points_fee, Uint128::new(10000));

    Ok(affiliate_fee_amount)
}

// QUERY HELPER FUNCTIONS

// Unexposed query helper function that queries the swap adapter contract to get the
// asset in needed for a given swap. Verifies the swap's in denom is the same as the
// swap asset denom from the message. Returns the swap asset in.
fn query_swap_asset_in(
    deps: &DepsMut,
    swap_adapter_contract: &ContractInfo,
    swap: &SwapExactAssetOut,
    swap_asset_out: &Asset,
) -> ContractResult<Asset> {
    // Query the swap adapter to get the asset in needed for the fee swap
    let fee_swap_asset_in: Asset = deps.querier.query_wasm_smart(
        swap_adapter_contract.address.clone(),
        swap_adapter_contract.code_hash.clone(),
        &SwapQueryMsg::SimulateSwapExactAssetIn {
            asset_in: swap_asset_out.clone(),
            swap_operations: swap.operations.clone(),
        },
    )?;

    Ok(fee_swap_asset_in)
}
