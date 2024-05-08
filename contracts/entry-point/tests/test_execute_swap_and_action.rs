use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_json_binary, Addr, BankMsg, Coin, ContractResult, OverflowError, OverflowOperation,
    QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg};
use cw_utils::PaymentError::{MultipleDenoms, NoFunds, NonPayable};
use skip::{
    asset::Asset,
    entry_point::{Action, Affiliate, ExecuteMsg},
    error::SkipError::{
        IbcFeesNotOneCoin, InvalidCw20Coin, Overflow, Payment, SwapOperationsAssetInDenomMismatch,
        SwapOperationsAssetOutDenomMismatch, SwapOperationsEmpty,
    },
    ibc::{IbcFee, IbcInfo},
    swap::{
        ExecuteMsg as SwapExecuteMsg, Route, SmartSwapExactAssetIn, Swap, SwapExactAssetIn,
        SwapExactAssetOut, SwapOperation,
    },
};
use skip_api_entry_point::{
    error::ContractError,
    state::{IBC_TRANSFER_CONTRACT_ADDRESS, PRE_SWAP_OUT_ASSET_AMOUNT, SWAP_VENUE_MAP},
};
use test_case::test_case;

/*
Test Cases:

Expect Response
    Native Asset
        - User Swap Exact Coin In With Transfer
        - User Swap Exact Coin Out With Transfer
        - User Swap Exact Coin In With IBC Transfer With IBC Fees
        - User Swap Exact Coin In With IBC Transfer Without IBC Fees
        - Fee Swap And User Swap Exact Coin In With IBC Fees
        - Sent Asset Not Given With Valid One Coin

    CW20 Asset
        - User Swap Exact Cw20 Asset In With Transfer
        - Fee Swap And User Swap Exact Cw20 Asset In With IBC Fees
        - Cw20 Min Asset

Expect Error
    // Fee Swap
    - Fee Swap Coin In Amount More Than Remaining Coin Received Amount
    - Fee Swap Coin In Denom Is Not The Same As Remaining Coin Received Denom
    - Fee Swap First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom
    - Fee Swap Last Swap Operation Denom Out Is Not The Same As IBC Fee Coin Denom
    - Fee Swap With IBC Transfer But Without IBC Fees

    // User Swap
    - User Swap With IBC Transfer With IBC Fees But IBC Fee Coin Denom Is Not The Same As Remaining Coin Received Denom

    // Invalid Assets Sent To Contract
    - No Coins Sent To Contract
    - More Than One Coin Sent To Contract
    - Not Enough Cw20 Tokens Sent To Contract
    - Cw20 Token Specified As Sent Asset With Native Coin Sent To Contract

    // Empty Swap Operations
    - Empty Fee Swap Operations

    // Timeout
    - Current Block Time Greater Than Timeout Timestamp

    // IBC Transfer
    - IBC Transfer With IBC Fees But More Than One IBC Fee Denom Specified
    - IBC Transfer With IBC Fees But No IBC Fee Coins Specified
    - IBC Transfer With IBC Fee Coin Amount Zero

    // Sent Asset
    - Sent Asset Not Given with Invalid One Coin
 */

// Define test parameters
struct Params {
    info_funds: Vec<Coin>,
    sent_asset: Option<Asset>,
    user_swap: Swap,
    min_asset: Asset,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap_and_action
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
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
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(1_000_000, "untrn")),
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
    "User Swap Exact Coin In With Transfer")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_json_binary(&ExecuteMsg::UserSwap {
                        swap: Swap::SwapExactAssetOut (
                            SwapExactAssetOut{
                                swap_venue_name: "swap_venue_name".to_string(),
                                operations: vec![
                                    SwapOperation {
                                        pool: "pool".to_string(),
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                                refund_address: Some("refund_address".to_string()),
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(1_000_000, "untrn")),
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
                        exact_out: true,
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
    "User Swap Exact Coin Out With Transfer")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
        timeout_timestamp: 101,
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
            fee_swap: None,
        },
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "ibc_transfer_adapter".to_string(),
                    amount: vec![Coin::new(200_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
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
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(800_000, "untrn")),
                        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
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
                        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
                        timeout_timestamp: 101,
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
                            fee_swap: None,
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
    "User Swap Exact Coin In With IBC Transfer With IBC Fees")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
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
        affiliates: vec![],
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
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(1_000_000, "untrn")),
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
    "User Swap Exact Coin In With IBC Transfer Without IBC Fees")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "osmo".to_string(),
                                denom_out: "untrn".to_string(),
                                interface: None,
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
                msg: BankMsg::Send {
                    to_address: "ibc_transfer_adapter".to_string(),
                    amount: vec![Coin::new(200_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
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
                                        pool: "pool_2".to_string(),
                                        denom_in: "osmo".to_string(),
                                        denom_out: "uatom".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(800_000, "osmo")),
                        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
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
                        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
                        timeout_timestamp: 101,
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
                            fee_swap: Some(
                                SwapExactAssetOut {
                                    swap_venue_name: "swap_venue_name".to_string(), 
                                    operations: vec![
                                        SwapOperation {
                                            pool: "pool".to_string(),
                                            denom_in: "osmo".to_string(),
                                            denom_out: "untrn".to_string(),
                                            interface: None,
                                        }
                                    ],
                                    refund_address: None,
                                }
                            ),
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
    "Fee Swap And User Swap Exact Coin In With IBC Fees")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: None,
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
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
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(1_000_000, "untrn")),
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
    "Sent Asset Not Given With Valid One Coin")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Some(Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })),
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
                        remaining_asset: Asset::Cw20(Cw20Coin {
                            address: "neutron123".to_string(),
                            amount: Uint128::from(1_000_000u128),
                        }),
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
    "User Swap Exact Cw20 Asset In With Transfer")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Some(Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name_2".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "neutron123".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "neutron123".to_string(), 
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: "swap_venue_adapter_2".to_string(), 
                        amount: Uint128::from(200_000u128),
                        msg: to_json_binary(&SwapExecuteMsg::Swap {
                            operations: vec![
                                SwapOperation {
                                    pool: "pool".to_string(),
                                    denom_in: "neutron123".to_string(),
                                    denom_out: "untrn".to_string(),
                                    interface: None,
                                }
                            ],
                        }).unwrap(),
                    }).unwrap(),
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "ibc_transfer_adapter".to_string(),
                    amount: vec![Coin::new(200_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
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
                                        pool: "pool_2".to_string(),
                                        denom_in: "neutron123".to_string(),
                                        denom_out: "uatom".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Cw20(Cw20Coin {
                            address: "neutron123".to_string(),
                            amount: Uint128::from(800_000u128),
                        }),
                        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
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
                        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
                        timeout_timestamp: 101,
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
                            fee_swap: Some(
                                SwapExactAssetOut {
                                    swap_venue_name: "swap_venue_name_2".to_string(), 
                                    operations: vec![
                                        SwapOperation {
                                            pool: "pool".to_string(),
                                            denom_in: "neutron123".to_string(),
                                            denom_out: "untrn".to_string(),
                                            interface: None,
                                        }
                                    ],
                                    refund_address: None,
                                }
                            ),
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
    "Fee Swap And User Swap Exact Cw20 Asset In With IBC Fees")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        min_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        }),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
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
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }
                                ],
                            }
                        ),
                        remaining_asset: Asset::Native(Coin::new(1_000_000, "untrn")),
                        min_asset: Asset::Cw20(Cw20Coin {
                            address: "neutron123".to_string(),
                            amount: Uint128::from(1_000_000u128),
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
                    msg: to_json_binary(&ExecuteMsg::PostSwapAction {
                        min_asset: Asset::Cw20(Cw20Coin {
                            address: "neutron123".to_string(),
                            amount: Uint128::from(1_000_000u128),
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
    "Cw20 Min Asset")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(100_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(100_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "100000".to_string(),
            operand2: "200000".to_string(),
        }))),
    };
    "Fee Swap Coin In Amount More Than Remaining Coin Received Amount- Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "uatom"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "uatom"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "uatom".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapAssetInDenomMismatch),
    };
    "Fee Swap Coin In Denom In Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "uatom".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsAssetInDenomMismatch)),
    };
    "Fee Swap First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsAssetOutDenomMismatch)),
    };
    "Fee Swap Last Swap Operation Denom Out Is Not The Same As IBC Fee Coin Denom- Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "uatom")],
                    timeout_fee: vec![Coin::new(100_000, "uatom")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: None,
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::IBCFeeDenomDiffersFromAssetReceived),
    };
    "User Swap With IBC Transfer With IBC Fees But IBC Fee Coin Denom Is Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "atom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "atom")),
        timeout_timestamp: 101,
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: None,
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::FeeSwapWithoutIbcFees),
    };
    "Fee Swap With IBC Trnasfer But Without IBC Fees - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "atom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "atom")),
        timeout_timestamp: 101,
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![Coin::new(100_000, "uatom")],
                    timeout_fee: vec![Coin::new(100_000, "untrn")],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(IbcFeesNotOneCoin)),
    };
    "IBC Transfer With IBC Fees But More Than One IBC Fee Denom Specified - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "atom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "atom")),
        timeout_timestamp: 101,
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![],
                    ack_fee: vec![],
                    timeout_fee: vec![],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(IbcFeesNotOneCoin)),
    };
    "IBC Transfer With IBC Fees But No IBC Fee Coins Specified - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "atom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "atom")),
        timeout_timestamp: 101,
        post_swap_action: Action::IbcTransfer {
            ibc_info: IbcInfo {
                source_channel: "channel-0".to_string(),
                receiver: "receiver".to_string(),
                memo: "".to_string(),
                fee: Some(IbcFee {
                    recv_fee: vec![Coin::new(0, "uatom")],
                    ack_fee: vec![],
                    timeout_fee: vec![],
                }),
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5"
                    .to_string(),
            },
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![
                        SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "osmo".to_string(),
                            denom_out: "untrn".to_string(),
                            interface: None,
                        }
                    ],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(IbcFeesNotOneCoin)),
    };
    "IBC Transfer With IBC Fee Coin Amount Zero - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Payment(NoFunds{}))),
    };
    "No Coins Sent to Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Payment(MultipleDenoms{}))),
    };
    "More Than One Coin Sent to Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Some(Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(2_000_000u128),
        })),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(InvalidCw20Coin)),
    };
    "Not Enough Cw20 Tokens Sent To Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "osmo")],
        sent_asset: Some(Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Payment(NonPayable{}))),
    };
    "Cw20 Token Specified As Sent Asset With Native Coin Sent To Contract - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "osmo"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "osmo"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "osmo".to_string(),
                        denom_out: "uatom".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        min_asset: Asset::Native(Coin::new(100_000, "uatom")),
        timeout_timestamp: 101,
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
            fee_swap: Some(
                SwapExactAssetOut {
                    swap_venue_name: "swap_venue_name".to_string(), 
                    operations: vec![],
                    refund_address: None,
                }
            ),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsEmpty)),
    };
    "Empty Fee Swap Operations - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![
            Coin::new(1_000_000, "untrn"),
        ],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![],
            },
        ),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 99,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Timeout),
    };
    "Current Block Time Greater Than Timeout Timestamp - Expect Error")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: None,
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
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
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(NoFunds{})),
    };
    "Sent Asset Not Given with Invalid One Coin")]
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "untrn")],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SmartSwapExactAssetIn(SmartSwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            routes: vec![
                Route {
                    offer_asset: Asset::Native(Coin::new(250_000, "untrn")),
                    operations: vec![SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "untrn".to_string(),
                        denom_out: "osmo".to_string(),
                        interface: None,
                    }],
                },
                Route {
                    offer_asset: Asset::Native(Coin::new(750_000, "untrn")),
                    operations: vec![
                        SwapOperation {
                            pool: "pool_2".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "neutron123".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "pool_3".to_string(),
                            denom_in: "neutron123".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        },
                    ],
                },
            ],
        }),
        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
        timeout_timestamp: 101,
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
                recover_address: "cosmos1xv9tklw7d82sezh9haa573wufgy59vmwe6xxe5".to_string(),
            },
            fee_swap: None,
        },
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "ibc_transfer_adapter".to_string(),
                    amount: vec![Coin::new(200_000, "untrn")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "entry_point".to_string(), 
                    msg: to_json_binary(&ExecuteMsg::UserSwap {
                        swap: Swap::SmartSwapExactAssetIn(SmartSwapExactAssetIn {
                            swap_venue_name: "swap_venue_name".to_string(),
                            routes: vec![
                                Route {
                                    offer_asset: Asset::Native(Coin::new(250_000, "untrn")),
                                    operations: vec![SwapOperation {
                                        pool: "pool".to_string(),
                                        denom_in: "untrn".to_string(),
                                        denom_out: "osmo".to_string(),
                                        interface: None,
                                    }],
                                },
                                Route {
                                    offer_asset: Asset::Native(Coin::new(550_000, "untrn")),
                                    operations: vec![
                                        SwapOperation {
                                            pool: "pool_2".to_string(),
                                            denom_in: "untrn".to_string(),
                                            denom_out: "neutron123".to_string(),
                                            interface: None,
                                        },
                                        SwapOperation {
                                            pool: "pool_3".to_string(),
                                            denom_in: "neutron123".to_string(),
                                            denom_out: "osmo".to_string(),
                                            interface: None,
                                        },
                                    ],
                                },
                            ],
                        }),
                        remaining_asset: Asset::Native(Coin::new(800_000, "untrn")),
                        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
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
                        min_asset: Asset::Native(Coin::new(800_000, "osmo")),
                        timeout_timestamp: 101,
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
                            fee_swap: None,
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
    "Adjusts SmartSwapExactAssetIn route offer_assets sum to match remaining_asset"
)]
fn test_execute_swap_and_action(params: Params) {
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
                } else if contract_addr == "swap_venue_adapter_2" {
                    SystemResult::Ok(ContractResult::Ok(
                        to_json_binary(&Asset::Cw20(Cw20Coin {
                            address: "neutron123".to_string(),
                            amount: Uint128::from(200_000u128),
                        }))
                        .unwrap(),
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
    let info = mock_info("swapper", info_funds);

    // Store the swap venue adapter contract address
    let swap_venue_adapter = Addr::unchecked("swap_venue_adapter");
    let swap_venue_adapter_2 = Addr::unchecked("swap_venue_adapter_2");
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name",
            &swap_venue_adapter,
        )
        .unwrap();
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name_2",
            &swap_venue_adapter_2,
        )
        .unwrap();

    // Store the ibc transfer adapter contract address
    let ibc_transfer_adapter = Addr::unchecked("ibc_transfer_adapter");
    IBC_TRANSFER_CONTRACT_ADDRESS
        .save(deps.as_mut().storage, &ibc_transfer_adapter)
        .unwrap();

    // Call execute_swap_and_action with the given test case params
    let res = skip_api_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SwapAndAction {
            sent_asset: params.sent_asset,
            user_swap: params.user_swap,
            min_asset: params.min_asset,
            timeout_timestamp: params.timeout_timestamp,
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

            // Assert the pre swap out asset amount set is correct
            let pre_swap_out_asset_amount = PRE_SWAP_OUT_ASSET_AMOUNT.load(&deps.storage).unwrap();
            assert_eq!(pre_swap_out_asset_amount, Uint128::from(1_000_000u128));
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
