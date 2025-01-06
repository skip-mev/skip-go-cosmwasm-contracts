use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_binary, Addr, Coin, ContractInfo, ContractResult, QuerierResult,
    ReplyOn::{Always, Never},
    StdError, SubMsg, SystemError, SystemResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use secret_skip::{
    asset::Asset,
    cw20::Cw20Coin,
    snip20::Snip20ReceiveMsg,
    swap::{Swap, SwapExactAssetIn, SwapOperation},
};
use secret_toolkit::snip20::{AuthenticatedQueryResponse, Balance};
use skip_go_secret_entry_point::{
    error::ContractError,
    msg::{Action, Affiliate, ExecuteMsg, Snip20HookMsg},
    reply::RECOVER_REPLY_ID,
    state::{IBC_TRANSFER_CONTRACT, REGISTERED_TOKENS, SWAP_VENUE_MAP, VIEWING_KEY},
};
use test_case::test_case;

#[cw_serde]
pub enum Snip20Response {
    Balance { amount: Uint128 },
}

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
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 1_000_000u128.into(),
        }),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn {
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "secret123".to_string(),
                        denom_out: "secret456".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        min_asset: Asset::Cw20(Cw20Coin {
            address: "secret456".to_string(),
            amount: 1000u128.into(),
        }),
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
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::UserSwap {
                        swap: Swap::SwapExactAssetIn (
                            SwapExactAssetIn{
                                swap_venue_name: "swap_venue_name".to_string(),
                                operations: vec![
                                    SwapOperation {
                                        pool: "pool".to_string(),
                                        denom_in: "secret123".to_string(),
                                        denom_out: "secret456".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Cw20(Cw20Coin {
                            address: "secret123".to_string(),
                            amount: 1_000_000u128.into(),
                        }),
                        min_asset: Asset::Cw20(Cw20Coin {
                            address: "secret456".to_string(),
                            amount: 1000u128.into(),
                        }),
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
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::PostSwapAction {
                        min_asset: Asset::Cw20(Cw20Coin {
                            address: "secret456".to_string(),
                            amount: 1000u128.into(),
                        }),
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
        sent_asset: Asset::Cw20(Cw20Coin{address: "secret123".to_string(), amount: 1_000_000u128.into()}),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "secret123".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        min_asset: Asset::Cw20(Cw20Coin {
            address: "secret456".to_string(),
            amount: 1000u128.into(),
        }),
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
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::SwapAndAction {
                        sent_asset: Some(Asset::Cw20(Cw20Coin{address: "secret123".to_string(), amount: 1_000_000u128.into()})),
                        user_swap: Swap::SwapExactAssetIn (
                            SwapExactAssetIn{
                                swap_venue_name: "swap_venue_name".to_string(),
                                operations: vec![
                                    SwapOperation {
                                        pool: "pool".to_string(),
                                        denom_in: "secret123".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        min_asset: Asset::Cw20(Cw20Coin {
                            address: "secret456".to_string(),
                            amount: 1000u128.into(),
                        }),
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
    let mut deps = mock_dependencies_with_balances(&[("entry_point", &[])]);

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "swap_venue_adapter" {
                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&Asset::Native(Coin::new(200_000, "osmo"))).unwrap(),
                    ))
                } else if vec!["secret123", "secret456"].contains(&contract_addr.as_str()) {
                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&Snip20Response::Balance {
                            amount: 1_000_000u128.into(),
                        })
                        .unwrap(),
                    ))
                } else {
                    SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: format!("query {}", contract_addr),
                    })
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
    env.contract.code_hash = "code_hash".to_string();
    env.block.time = Timestamp::from_nanos(100);

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info("secret123", info_funds);

    // Store the swap venue adapter contract address
    let swap_venue_adapter = Addr::unchecked("swap_venue_adapter");
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name",
            &ContractInfo {
                address: swap_venue_adapter,
                code_hash: "code_hash".to_string(),
            },
        )
        .unwrap();

    // Store the ibc transfer adapter contract address
    let ibc_transfer_adapter = ContractInfo {
        address: Addr::unchecked("ibc_transfer_adapter"),
        code_hash: "code_hash".to_string(),
    };
    IBC_TRANSFER_CONTRACT
        .save(deps.as_mut().storage, &ibc_transfer_adapter)
        .unwrap();

    REGISTERED_TOKENS
        .save(
            deps.as_mut().storage,
            Addr::unchecked("secret123"),
            &ContractInfo {
                address: Addr::unchecked("secret123"),
                code_hash: "code_hash".to_string(),
            },
        )
        .unwrap();
    REGISTERED_TOKENS
        .save(
            deps.as_mut().storage,
            Addr::unchecked(params.min_asset.clone().denom()),
            &ContractInfo {
                address: Addr::unchecked(params.min_asset.clone().denom()),
                code_hash: "code_hash".to_string(),
            },
        )
        .unwrap();

    VIEWING_KEY
        .save(deps.as_mut().storage, &"viewing_key".to_string())
        .unwrap();

    // Call execute_receive with the given test case params
    let res = match params.recovery_addr {
        Some(recovery_addr) => skip_go_secret_entry_point::contract::execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::Receive(Snip20ReceiveMsg {
                sender: Addr::unchecked("swapper".to_string()),
                amount: params.sent_asset.amount(),
                from: Addr::unchecked("swapper".to_string()),
                memo: None,
                msg: Some(
                    to_binary(&Snip20HookMsg::SwapAndActionWithRecover {
                        user_swap: params.user_swap,
                        min_asset: params.min_asset.clone(),
                        timeout_timestamp: params.timeout_timestamp,
                        post_swap_action: params.post_swap_action,
                        affiliates: params.affiliates,
                        recovery_addr,
                    })
                    .unwrap(),
                ),
            }),
        ),
        None => skip_go_secret_entry_point::contract::execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::Receive(Snip20ReceiveMsg {
                sender: Addr::unchecked("swapper".to_string()),
                amount: params.sent_asset.amount(),
                from: Addr::unchecked("swapper".to_string()),
                memo: None,
                msg: Some(
                    to_binary(&Snip20HookMsg::SwapAndAction {
                        user_swap: params.user_swap,
                        min_asset: params.min_asset,
                        timeout_timestamp: params.timeout_timestamp,
                        post_swap_action: params.post_swap_action,
                        affiliates: params.affiliates,
                    })
                    .unwrap(),
                ),
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
