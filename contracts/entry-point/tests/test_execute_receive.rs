use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_json_binary, Addr, Coin, ContractResult, QuerierResult,
    ReplyOn::{Always, Never},
    SubMsg, SystemResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg};
use skip::{
    asset::Asset,
    entry_point::{Action, Affiliate, Cw20HookMsg, ExecuteMsg},
    swap::{Swap, SwapExactAssetIn, SwapOperation},
};
use skip_api_entry_point::{
    error::ContractError,
    reply::RECOVER_REPLY_ID,
    state::{IBC_TRANSFER_CONTRACT_ADDRESS, SWAP_VENUE_MAP},
};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Valid Swap And Action Msg
    - Valid Swap And Action With Recover Msg
 */

// Define test parameters
struct Params {
    info_funds: Vec<Coin>,
    sent_asset: Asset,
    user_swap: Swap,
    min_asset: Asset,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
    recovery_addr: Option<Addr>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_receive
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Asset::Cw20(Cw20Coin{address: "neutron123".to_string(), amount: Uint128::from(1_000_000u128)}),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        recovery_addr: None,
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_json_binary(&ExecuteMsg::UserSwap {
                        swap: Swap::SwapExactAssetIn (
                            SwapExactAssetIn{
                                swap_venue_name: "swap_venue_name".to_string(),
                                operations: vec![
                                    SwapOperation {
                                        pool: "pool".to_string(),
                                        denom_in: "neutron123".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Cw20(Cw20Coin{address: "neutron123".to_string(), amount: Uint128::from(1_000_000u128)}),
                        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                        affiliates: vec![],
                    }).unwrap(),
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_json_binary(&ExecuteMsg::PostSwapAction {
                        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                        timeout_timestamp: 101,
                        post_swap_action: Action::Transfer {
                            to_address: "to_address".to_string(),
                        },
                        exact_out: false,
                    }).unwrap(),
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Valid Swap And Action Msg")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Asset::Cw20(Cw20Coin{address: "neutron123".to_string(), amount: Uint128::from(1_000_000u128)}),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        recovery_addr: Some(Addr::unchecked("recovery_addr")),
        expected_messages: vec![
            SubMsg {
                id: RECOVER_REPLY_ID,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                        sent_asset: Some(Asset::Cw20(Cw20Coin{address: "neutron123".to_string(), amount: Uint128::from(1_000_000u128)})),
                        user_swap: Swap::SwapExactAssetIn (
                            SwapExactAssetIn{
                                swap_venue_name: "swap_venue_name".to_string(),
                                operations: vec![
                                    SwapOperation {
                                        pool: "pool".to_string(),
                                        denom_in: "neutron123".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                        timeout_timestamp: 101,
                        post_swap_action: Action::Transfer {
                            to_address: "to_address".to_string(),
                        },
                        affiliates: vec![] })
                        .unwrap(),
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Always,
            },
        ],
        expected_error: None,
    };
    "Valid Swap And Action With Recover Msg")]
fn test_execute_receive(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "swap_venue_adapter" {
                    SystemResult::Ok(ContractResult::Ok(
                        to_json_binary(&Asset::Native(Coin::new(200_000, "osmo"))).unwrap(),
                    ))
                } else {
                    SystemResult::Ok(ContractResult::Ok(
                        to_json_binary(&BalanceResponse {
                            balance: Uint128::from(1_000_000u128),
                        })
                        .unwrap(),
                    ))
                }
            }
            _ => panic!("Unsupported query: {:?}", query),
        }
    };

    // Update querier with mock wasm handler
    deps.querier.update_wasm(wasm_handler);

    // Create mock env with parameters that make testing easier
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("entry_point");
    env.block.time = Timestamp::from_nanos(100);

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info("neutron123", info_funds);

    // Store the swap venue adapter contract address
    let swap_venue_adapter = Addr::unchecked("swap_venue_adapter");
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name",
            &swap_venue_adapter,
        )
        .unwrap();

    // Store the ibc transfer adapter contract address
    let ibc_transfer_adapter = Addr::unchecked("ibc_transfer_adapter");
    IBC_TRANSFER_CONTRACT_ADDRESS
        .save(deps.as_mut().storage, &ibc_transfer_adapter)
        .unwrap();

    // Call execute_receive with the given test case params
    let res = match params.recovery_addr {
        Some(recovery_addr) => skip_api_entry_point::contract::execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "swapper".to_string(),
                amount: params.sent_asset.amount(),
                msg: to_json_binary(&Cw20HookMsg::SwapAndActionWithRecover {
                    user_swap: params.user_swap,
                    min_asset: params.min_asset,
                    timeout_timestamp: params.timeout_timestamp,
                    post_swap_action: params.post_swap_action,
                    affiliates: params.affiliates,
                    recovery_addr,
                })
                .unwrap(),
            }),
        ),
        None => skip_api_entry_point::contract::execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "swapper".to_string(),
                amount: params.sent_asset.amount(),
                msg: to_json_binary(&Cw20HookMsg::SwapAndAction {
                    user_swap: params.user_swap,
                    min_asset: params.min_asset,
                    timeout_timestamp: params.timeout_timestamp,
                    post_swap_action: params.post_swap_action,
                    affiliates: params.affiliates,
                })
                .unwrap(),
            }),
        ),
    };

    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the number of messages in the response is correct
            assert_eq!(
                res.messages.len(),
                params.expected_messages.len(),
                "expected {:?} messages, but got {:?}",
                params.expected_messages.len(),
                res.messages.len()
            );

            // Assert the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages,);
        }
        Err(err) => {
            // Assert the test expected an error
            assert!(
                params.expected_error.is_some(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert_eq!(err, params.expected_error.unwrap());
        }
    }
}
