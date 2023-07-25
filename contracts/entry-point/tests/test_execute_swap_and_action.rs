use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_binary, Addr, Coin, ContractResult, OverflowError, OverflowOperation, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Timestamp, WasmMsg, WasmQuery,
};
use cw_utils::PaymentError::{MultipleDenoms, NoFunds};
use skip::{
    entry_point::{Affiliate, ExecuteMsg, PostSwapAction},
    ibc::{IbcFee, IbcInfo},
    swap::{ExecuteMsg as SwapExecuteMsg, SwapExactCoinIn, SwapExactCoinOut, SwapOperation},
};
use skip_swap_entry_point::{error::ContractError, state::SWAP_VENUE_MAP};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - User Swap
    - Fee Swap And User Swap Using Leftover Coin
    - Fee Swap And User Swap Using Specified Coin

Expect Error
    // Fee Swap
    - Fee Swap Necessary Coin More Than Sent To Contract
    - Fee Swap Required Denom In Not The Same As Coin Sent To Contract
    - Fee Swap Coin Out Denom Is Not The Same As Last Swap Operation Denom Out
    - Fee Swap Without IBC Transfer Post Swap Action
    - Fee Swap Coin Out Greater Than Ibc Fee Requires
    TODO: Add fee swap exact coin in not equal to received

    // User Swap
    - User Swap Denom In Is Not The Same As First Swap Operation Denom In
    - User Swap Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom
    TODO: Add user swap exact coin in not equal to received

    // Invalid Coins Sent To Contract
    - No Coins Sent To Contract
    - More Than One Coin Sent To Contract

    // Empty Swap Operations
    - Empty User Swap Operations
    - Empty Fee Swap Operations
 */

// Define test parameters
struct Params {
    info_funds: Vec<Coin>,
    fee_swap: Option<SwapExactCoinOut>,
    user_swap: SwapExactCoinIn,
    min_coin: Coin,
    timeout_timestamp: u64,
    post_swap_action: PostSwapAction,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap_and_action
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        fee_swap: None,
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![
                SwapOperation {
                    pool: "pool".to_string(),
                    denom_in: "untrn".to_string(),
                    denom_out: "osmo".to_string(),
                }
            ],
        },
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 101,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "untrn".to_string(),
                                denom_out: "osmo".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "untrn")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_binary(&ExecuteMsg::PostSwapAction {
                        min_coin: Coin::new(1_000_000, "osmo"),
                        timeout_timestamp: 101,
                        post_swap_action: PostSwapAction::BankSend {
                            to_address: "to_address".to_string(),
                        },
                        affiliates: vec![],
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
    "User Swap")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "untrn".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "osmo".to_string(),
                                denom_out: "untrn".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(200_000, "osmo")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool_2".to_string(),
                                denom_in: "osmo".to_string(),
                                denom_out: "uatom".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(800_000, "osmo")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_binary(&ExecuteMsg::PostSwapAction {
                        min_coin: Coin::new(100_000, "uatom"),
                        timeout_timestamp: 101,
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
    "Fee Swap And User Swap Using Leftover Coin")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "untrn".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(800_000, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "osmo".to_string(),
                                denom_out: "untrn".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(200_000, "osmo")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool_2".to_string(),
                                denom_in: "osmo".to_string(),
                                denom_out: "uatom".to_string(),
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(800_000, "osmo")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_binary(&ExecuteMsg::PostSwapAction {
                        min_coin: Coin::new(100_000, "uatom"),
                        timeout_timestamp: 101,
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
    "Fee Swap And User Swap Using Specified Coin")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(100_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "untrn".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(100_000, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![],
        expected_error: Some(ContractError::Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "100000".to_string(),
            operand2: "200000".to_string(),
        })),
    };
    "Fee Swap Necessary Coin More Than Sent To Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "uatom"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "untrn".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(900_000, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapCoinInDenomMismatch),
    };
    "Fee Swap Required Denom In Not The Same As Coin Sent To Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "uatom".to_string(),
                        denom_out: "osmo".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(900_000, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapOperationsCoinOutDenomMismatch),
    };
    "Fee Swap Coin Out Denom Is Not The Same As Last Swap Operation Denom Out - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_001, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "untrn".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(799_999, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapCoinOutGreaterThanIbcFee),
    };
    "Fee Swap Coin Out Greater Than Ibc Fee Requires")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: None,
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(1_000_000, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "untrn".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![],
        expected_error: Some(ContractError::SwapOperationsCoinInDenomMismatch),
    };
    "User Swap Denom In Is Not The Same As First Swap Operation Denom In - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "untrn"),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "untrn".to_string(),
                    }
                ],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "atom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "atom"),
        timeout_timestamp: 101,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapNotAllowed),
    };
    "Fee Swap Without IBC Transfer Post Swap Action - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![],
        fee_swap: None,
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![
                SwapOperation {
                    pool: "pool".to_string(),
                    denom_in: "untrn".to_string(),
                    denom_out: "osmo".to_string(),
                }
            ],
        },
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 101,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(NoFunds{})),
    };
    "No Coins Sent to Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: None,
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![
                SwapOperation {
                    pool: "pool".to_string(),
                    denom_in: "untrn".to_string(),
                    denom_out: "osmo".to_string(),
                }
            ],
        },
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 101,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(MultipleDenoms{})),
    };
    "More Than One Coin Sent to Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        fee_swap: None,
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![],
        },
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 101,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![],
        expected_error: Some(ContractError::SwapOperationsEmpty),
    };
    "Empty User Swap Operations - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        fee_swap: Some(
            SwapExactCoinOut {
                swap_venue_name: "swap_venue_name".to_string(), 
                coin_out: Coin::new(200_000, "osmo"),
                operations: vec![],
            }
        ),
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: Some(Coin::new(900_000, "osmo")),
            operations: vec![
                SwapOperation {
                    pool: "pool_2".to_string(),
                    denom_in: "osmo".to_string(),
                    denom_out: "uatom".to_string(),
                }
            ],
        },
        min_coin: Coin::new(100_000, "uatom"),
        timeout_timestamp: 101,
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
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapOperationsEmpty),
    };
    "Empty Fee Swap Operations - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        fee_swap: None,
        user_swap: SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            coin_in: None,
            operations: vec![],
        },
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 99,
        post_swap_action: PostSwapAction::BankSend {
            to_address: "to_address".to_string(),
        },
        expected_messages: vec![],
        expected_error: Some(ContractError::Timeout),
    };
    "Current Block Time Greater Than Timeout Timestamp - Expect Error")]
fn test_execute_post_swap_action(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock wasm handler to handle the swap adapter contract query
    // Will always return 200_000 osmo
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&Coin::new(200_000, "osmo")).unwrap(),
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

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info("swapper", info_funds);

    // Store the ibc transfer adapter contract address
    let swap_venue_adapter = Addr::unchecked("swap_venue_adapter");
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name",
            &swap_venue_adapter,
        )
        .unwrap();

    // Create standardized params used across all tests
    // The reason for these params to not need to be defined in each test case is because
    // they have no direct impact on the test case
    let affiliates: Vec<Affiliate> = vec![];

    // Call execute_swap_and_action with the given test case params
    let res = skip_swap_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SwapAndAction {
            fee_swap: params.fee_swap,
            user_swap: params.user_swap,
            min_coin: params.min_coin,
            timeout_timestamp: params.timeout_timestamp,
            post_swap_action: params.post_swap_action,
            refund_action: None,
            affiliates,
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
