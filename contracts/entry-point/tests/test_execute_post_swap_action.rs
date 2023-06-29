use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin,
    ReplyOn::Never,
    SubMsg, Timestamp, Uint128, WasmMsg,
};
use skip::{
    entry_point::{Affiliate, ExecuteMsg, PostSwapAction},
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcFee, IbcInfo},
};
use skip_swap_entry_point::{error::ContractError, state::IBC_TRANSFER_CONTRACT_ADDRESS};
use test_case::test_case;

/*
Test Cases:

Expect Response
    // General
    - Bank Send
    - Ibc Transfer
    - Contract Call

    // With IBC Fees
    - Ibc Transfer w/ IBC Fees of different denom than min coin
    - Ibc Transfer w/ IBC Fees of same denom as min coin

    // With Affiliates
    - Bank Send w/ Affiliate
    - Contract Call w/ Affiliate
    - Ibc Transfer w/ IBC Fees of different denom than min coin w/ Affiliate
    - Ibc Transfer w/ IBC Fees of same denom as min coin w/ Affiliate

Expect Error
    - Bank Send Timeout
    - Ibc Transfer w/ Affiliates Decreasing user transfer below min coin
    - Ibc Transfer w/ IBC Fees Decreasing user transfer below min coin
    - Received Less From Swap Than Min Coin
    - Unauthorized Caller
 */

// Define test parameters
struct Params {
    caller: String,
    min_coin: Coin,
    post_swap_action: PostSwapAction,
    affiliates: Vec<Affiliate>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_post_swap_action
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: PostSwapAction::BankSend {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
                amount: vec![Coin::new(1_000_000, "osmo")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Bank Send")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![],
                    timeout_fee: vec![],
                },
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![],
                            timeout_fee: vec![],
                        },
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "osmo"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(1_000_000, "osmo")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Ibc Transfer")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: PostSwapAction::ContractCall {
            contract_address: "contract_call_address".to_string(),
            msg: to_binary(&"contract_call_msg").unwrap(),
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "contract_call_address".to_string(),
                msg: to_binary(&"contract_call_msg").unwrap(),
                funds: vec![Coin::new(1_000_000, "osmo")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Contract Call"
)]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                },
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "untrn")],
                            timeout_fee: vec![Coin::new(100_000, "untrn")],
                        },
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "osmo"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![
                    Coin::new(1_000_000, "osmo"),
                    Coin::new(200_000, "untrn"),
                ],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Ibc Transfer w/ IBC Fees of different denom than min coin")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(800_000, "untrn"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                },
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "untrn")],
                            timeout_fee: vec![Coin::new(100_000, "untrn")],
                        },
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(800_000, "untrn"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(1_000_000, "untrn")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Ibc Transfer w/ IBC Fees of same denom as min coin")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(900_000, "osmo"),
        post_swap_action: PostSwapAction::BankSend {
            to_address: "swapper".to_string(),
        },
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![Coin::new(900_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Bank Send w/ Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(900_000, "osmo"),
        post_swap_action: PostSwapAction::ContractCall {
            contract_address: "contract_call".to_string(),
            msg: to_binary(&"contract_call_msg").unwrap(),
        },
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "contract_call".to_string(),
                    msg: to_binary(&"contract_call_msg").unwrap(),
                    funds: vec![Coin::new(900_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Contract Call w/ Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(900_000, "osmo"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                },
                recover_address: "recover".to_string(),
            },
        },
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "osmo")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "ibc_transfer_adapter".to_string(),
                    msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                        info: IbcInfo {
                            source_channel: "channel-0".to_string(),
                            receiver: "receiver".to_string(),
                            memo: "".to_string(),
                            fee: IbcFee {
                                recv_fee: vec![],
                                ack_fee: vec![Coin::new(100_000, "untrn")],
                                timeout_fee: vec![Coin::new(100_000, "untrn")],
                            },
                            recover_address: "recover".to_string(),
                        },
                        coin: Coin::new(900_000, "osmo"),
                        timeout_timestamp: 101,
                    })
                    .unwrap(),
                    funds: vec![Coin::new(900_000, "osmo"), Coin::new(200_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Ibc Transfer w/ IBC Fees of different denom than min coin w/ Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(700_000, "untrn"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                },
                recover_address: "recover".to_string(),
            },
        },
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "ibc_transfer_adapter".to_string(),
                    msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                        info: IbcInfo {
                            source_channel: "channel-0".to_string(),
                            receiver: "receiver".to_string(),
                            memo: "".to_string(),
                            fee: IbcFee {
                                recv_fee: vec![],
                                ack_fee: vec![Coin::new(100_000, "untrn")],
                                timeout_fee: vec![Coin::new(100_000, "untrn")],
                            },
                            recover_address: "recover".to_string(),
                        },
                        coin: Coin::new(700_000, "untrn"),
                        timeout_timestamp: 101,
                    })
                    .unwrap(),
                    funds: vec![Coin::new(900_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Ibc Transfer w/ IBC Fees of same denom as min coin w/ Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(950_000, "osmo"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                },
                recover_address: "recover".to_string(),
            },
        },
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![],
        expected_error: Some(ContractError::TransferOutCoinLessThanMinAfterAffiliateFees),
    };
    "Ibc Transfer w/ Affiliates Decreasing user transfer below min coin - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(900_000, "untrn"),
        post_swap_action: PostSwapAction::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                },
                recover_address: "recover".to_string(),
            },
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::TransferOutCoinLessThanMinAfterIbcFees),
    };
    "Ibc Transfer w/ IBC Fees Decreasing user transfer below min coin - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_100_000, "untrn"),
        post_swap_action: PostSwapAction::BankSend {
            to_address: "swapper".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::ReceivedLessCoinFromSwapsThanMinCoin),
    };
    "Received Less From Swap Than Min Coin - Expect Error")]
#[test_case(
    Params {
        caller: "unauthorized".to_string(),
        min_coin: Coin::new(1_100_000, "untrn"),
        post_swap_action: PostSwapAction::BankSend {
            to_address: "swapper".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized {}),
    };
    "Unauthorized Caller - Expect Error")]
fn test_execute_post_swap_action(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock env with parameters that make testing easier
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("entry_point");
    env.block.time = Timestamp::from_nanos(100);

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, &[]);

    // Store the ibc transfer adapter contract address
    let ibc_transfer_adapter = Addr::unchecked("ibc_transfer_adapter");
    IBC_TRANSFER_CONTRACT_ADDRESS
        .save(deps.as_mut().storage, &ibc_transfer_adapter)
        .unwrap();

    // Call execute_post_swap_action with the given test parameters
    let res = skip_swap_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::PostSwapAction {
            min_coin: params.min_coin,
            timeout_timestamp: 101,
            post_swap_action: params.post_swap_action,
            affiliates: params.affiliates,
        },
    );

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
