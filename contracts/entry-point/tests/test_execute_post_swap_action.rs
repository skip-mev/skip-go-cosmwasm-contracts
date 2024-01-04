use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_json_binary, Addr, BankMsg, Coin, ContractResult, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg};
use skip::{
    asset::Asset,
    entry_point::{Action, ExecuteMsg},
    ibc::{ExecuteMsg as IbcTransferExecuteMsg, IbcFee, IbcInfo},
};
use skip_api_entry_point::{
    error::ContractError,
    state::{BLOCKED_CONTRACT_ADDRESSES, IBC_TRANSFER_CONTRACT_ADDRESS, PRE_SWAP_OUT_ASSET_AMOUNT},
};
use test_case::test_case;

/*
Test Cases:

Expect Response
    // General
    - Native Asset Transfer
    - Cw20 Asset Transfer
    - Ibc Transfer
    - Native Asset Contract Call
    - Cw20 Asset Contract Call

    // With IBC Fees
    - Ibc Transfer w/ IBC Fees of different denom than min coin
    - Ibc Transfer w/ IBC Fees of same denom as min coin

    // Exact Out
    - Native Asset Transfer With Exact Out Set To True
    - Ibc Transfer With Exact Out Set To True
    - Ibc Transfer w/ IBC Fees of different denom than min coin With Exact Out Set To True
    - Ibc Transfer w/ IBC Fees of same denom as min coin With Exact Out Set To True
    - Contract Call With Exact Out Set To True

    // Invariants
    - Pre Swap Out Asset Contract Balance Preserved

Expect Error
    - Transfer Timeout
    - Received Less Native Asset From Swap Than Min Asset
    - Received Less Cw20 Asset From Swap Than Min Asset
    - Unauthorized Caller
    - Contract Call Address Blocked
    - Cw20 Out Asset With IBC Transfer
 */

// Define test parameters
struct Params {
    caller: String,
    min_asset: Asset,
    post_swap_action: Action,
    exact_out: bool,
    pre_swap_out_asset_amount: Uint128,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_post_swap_action
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Native(Coin::new(100_000, "os")),
        post_swap_action: Action::Transfer {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        exact_out: true,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
                amount: vec![Coin::new(100_000, "os")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Native Asset Transfer With Exact Out Set To True")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        post_swap_action: Action::Transfer {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
                amount: vec![Coin::new(1_000_000, "os")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Native Asset Transfer")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::new(1_000_000),
        }),
        post_swap_action: Action::Transfer {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "neutron123".to_string(), 
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
                    amount: Uint128::new(1_000_000),
                }).unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Cw20 Asset Transfer")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: None,
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_json_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: None,
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "os"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(1_000_000, "os")],
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
        min_asset: Asset::Native(Coin::new(100_000, "os")),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: None,
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: true,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_json_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: None,
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(100_000, "os"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(100_000, "os")],
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
        min_asset: Asset::Native(Coin::new(100_000, "os")),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "un")],
                    timeout_fee: vec![Coin::new(100_000, "un")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: true,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_json_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "un")],
                            timeout_fee: vec![Coin::new(100_000, "un")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(100_000, "os"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![
                    Coin::new(100_000, "os"),
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
        min_asset: Asset::Native(Coin::new(100_000, "un")),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "un")],
                    timeout_fee: vec![Coin::new(100_000, "un")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: true,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_json_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "un")],
                            timeout_fee: vec![Coin::new(100_000, "un")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(100_000, "un"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(100_000, "un")],
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
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        post_swap_action: Action::ContractCall {
            contract_address: "contract_call".to_string(),
            msg: to_json_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "contract_call".to_string(),
                msg: to_json_binary(&"contract_call_msg").unwrap(),
                funds: vec![Coin::new(1_000_000, "os")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Native Asset Contract Call")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::new(1_000_000),
        }),
        post_swap_action: Action::ContractCall {
            contract_address: "contract_call".to_string(),
            msg: to_json_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "neutron123".to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: "contract_call".to_string(),
                    amount: Uint128::new(1_000_000),
                    msg: to_json_binary(&"contract_call_msg").unwrap(),
                }).unwrap(),
                funds: vec![],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Cw20 Asset Contract Call")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Native(Coin::new(100_000, "os")),
        post_swap_action: Action::ContractCall {
            contract_address: "contract_call".to_string(),
            msg: to_json_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: true,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "contract_call".to_string(),
                msg: to_json_binary(&"contract_call_msg").unwrap(),
                funds: vec![Coin::new(100_000, "os")],
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
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "un")],
                    timeout_fee: vec![Coin::new(100_000, "un")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_json_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "un")],
                            timeout_fee: vec![Coin::new(100_000, "un")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "os"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![
                    Coin::new(1_000_000, "os"),
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
        min_asset: Asset::Native(Coin::new(800_000, "un")),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "un")],
                    timeout_fee: vec![Coin::new(100_000, "un")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "ibc_transfer_adapter".to_string(),
                msg: to_json_binary(&IbcTransferExecuteMsg::IbcTransfer {
                    info: IbcInfo {
                        source_channel: "channel-0".to_string(),
                        receiver: "receiver".to_string(),
                        memo: "".to_string(),
                        fee: Some(IbcFee {
                            recv_fee: vec![],
                            ack_fee: vec![Coin::new(100_000, "un")],
                            timeout_fee: vec![Coin::new(100_000, "un")],
                        }),
                        recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                            .to_string(),
                    },
                    coin: Coin::new(1_000_000, "un"),
                    timeout_timestamp: 101,
                })
                .unwrap(),
                funds: vec![Coin::new(1_000_000, "un")],
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
        min_asset: Asset::Native(Coin::new(700_000, "os")),
        post_swap_action: Action::Transfer {
            to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(200_000),
        expected_messages: vec![SubMsg {
            id: 0,
            msg: BankMsg::Send {
                to_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
                amount: vec![Coin::new(800_000, "os")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        }],
        expected_error: None,
    };
    "Pre Swap Out Asset Contract Balance Preserved")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::new(1_000_000),
        }),
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: None,
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![],
        expected_error: Some(ContractError::NonNativeIbcTransfer),
    };
    "Cw20 Out Asset With IBC Transfer")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Native(Coin::new(1_100_000, "un")),
        post_swap_action: Action::Transfer {
            to_address: "swapper".to_string(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![],
        expected_error: Some(ContractError::ReceivedLessAssetFromSwapsThanMinAsset),
    };
    "Received Less Native Asset From Swap Than Min Asset - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::new(2_100_000),
        }),
        post_swap_action: Action::Transfer {
            to_address: "swapper".to_string(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![],
        expected_error: Some(ContractError::ReceivedLessAssetFromSwapsThanMinAsset),
    };
    "Received Less Cw20 Asset From Swap Than Min Asset - Expect Error")]
#[test_case(
    Params {
        caller: "unauthorized".to_string(),
        min_asset: Asset::Native(Coin::new(1_100_000, "un")),
        post_swap_action: Action::Transfer {
            to_address: "swapper".to_string(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        min_asset: Asset::Native(Coin::new(900_000, "un")),
        post_swap_action: Action::ContractCall {
            contract_address: "entry_point".to_string(),
            msg: to_json_binary(&"contract_call_msg").unwrap(),
        },
        exact_out: false,
        pre_swap_out_asset_amount: Uint128::new(0),
        expected_messages: vec![],
        expected_error: Some(ContractError::ContractCallAddressBlocked),
    };
    "Contract Call Address Blocked - Expect Error")]
fn test_execute_post_swap_action(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "os"), Coin::new(1_000_000, "un")],
    )]);

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { .. } => SystemResult::Ok(ContractResult::Ok(
                to_json_binary(&BalanceResponse {
                    balance: Uint128::from(1_000_000u128),
                })
                .unwrap(),
            )),
            _ => panic!("Unsupported query: {:?}", query),
        }
    };

    // Update querier with mock wasm handler
    deps.querier.update_wasm(wasm_handler);

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

    // Store the pre swap out asset amount
    PRE_SWAP_OUT_ASSET_AMOUNT
        .save(deps.as_mut().storage, &params.pre_swap_out_asset_amount)
        .unwrap();

    // Call execute_post_swap_action with the given test parameters
    let res = skip_api_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::PostSwapAction {
            min_asset: params.min_asset,
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
