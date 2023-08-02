use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin,
    ReplyOn::Never,
    SubMsg, Timestamp, WasmMsg,
};
use skip::{
    entry_point::{Action, ExecuteMsg},
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcFee, IbcInfo},
};
use skip_swap_entry_point::{
    error::ContractError,
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS},
};
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

    // Exact Out
    - Bank Send With Exact Out Set To True
    - Ibc Transfer With Exact Out Set To True
    - Ibc Transfer w/ IBC Fees of different denom than min coin With Exact Out Set To True
    - Ibc Transfer w/ IBC Fees of same denom as min coin With Exact Out Set To True
    - Contract Call With Exact Out Set To True

Expect Error
    - Bank Send Timeout
    - Received Less From Swap Than Min Coin
    - Unauthorized Caller
    - Contract Call Address Blocked
 */

// Define test parameters
struct Params {
    caller: String,
    min_coin: Coin,
    post_swap_action: Action,
    exact_out: bool,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_post_swap_action
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(100_000, "osmo"),
        post_swap_action: Action::BankSend {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        exact_out: true,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
                amount: vec![Coin::new(100_000, "osmo")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Bank Send With Exact Out Set To True")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: Action::BankSend {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        exact_out: false,
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
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: None,
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        exact_out: false,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: None,
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
        min_coin: Coin::new(100_000, "osmo"),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: None,
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        exact_out: true,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: None,
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(100_000, "osmo"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(100_000, "osmo")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Ibc Transfer With Exact Out Set To True")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(100_000, "osmo"),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        exact_out: true,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "untrn")],
                            timeout_fee: vec![Coin::new(100_000, "untrn")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(100_000, "osmo"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![
                    Coin::new(100_000, "osmo"),
                ],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Ibc Transfer w/ IBC Fees of different denom than min coin With Exact Out Set To True")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(100_000, "untrn"),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        exact_out: true,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "untrn")],
                            timeout_fee: vec![Coin::new(100_000, "untrn")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(100_000, "untrn"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(100_000, "untrn")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Ibc Transfer w/ IBC Fees of same denom as min coin With Exact Out Set To True")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: Action::ContractCall {
            contract_address: "contract_call".to_string(),
            msg: to_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: false,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "contract_call".to_string(),
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
        min_coin: Coin::new(100_000, "osmo"),
        post_swap_action: Action::ContractCall {
            contract_address: "contract_call".to_string(),
            msg: to_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: true,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "contract_call".to_string(),
                msg: to_binary(&"contract_call_msg").unwrap(),
                funds: vec![Coin::new(100_000, "osmo")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Contract Call With Exact Out Set To True")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(1_000_000, "osmo"),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        exact_out: false,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "untrn")],
                            timeout_fee: vec![Coin::new(100_000, "untrn")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "osmo"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![
                    Coin::new(1_000_000, "osmo"),
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
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "untrn")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
        },
        exact_out: false,
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "untrn")],
                            timeout_fee: vec![Coin::new(100_000, "untrn")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "untrn"),
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
        min_coin: Coin::new(1_100_000, "untrn"),
        post_swap_action: Action::BankSend {
            to_address: "swapper".to_string(),
        },
        exact_out: false,
        expected_messages: vec![],
        expected_error: Some(ContractError::ReceivedLessCoinFromSwapsThanMinCoin),
    };
    "Received Less From Swap Than Min Coin - Expect Error")]
#[test_case(
    Params {
        caller: "unauthorized".to_string(),
        min_coin: Coin::new(1_100_000, "untrn"),
        post_swap_action: Action::BankSend {
            to_address: "swapper".to_string(),
        },
        exact_out: false,
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_coin: Coin::new(900_000, "untrn"),
        post_swap_action: Action::ContractCall {
            contract_address: "entry_point".to_string(),
            msg: to_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: false,
        expected_messages: vec![],
        expected_error: Some(ContractError::ContractCallAddressBlocked),
    };
    "Contract Call Address Blocked - Expect Error")]
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

    // Store the entry point contract address in the blocked contract addresses map
    BLOCKED_CONTRACT_ADDRESSES
        .save(deps.as_mut().storage, &Addr::unchecked("entry_point"), &())
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
            exact_out: params.exact_out,
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
