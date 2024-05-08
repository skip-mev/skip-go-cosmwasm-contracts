use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_json_binary, Addr, BankMsg, Coin, ContractResult, OverflowError, OverflowOperation,
    QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use skip::{
    asset::Asset,
    entry_point::{Affiliate, ExecuteMsg},
    error::SkipError::{
        Overflow, SwapOperationsAssetInDenomMismatch, SwapOperationsAssetOutDenomMismatch,
        SwapOperationsEmpty,
    },
    swap::{
        ExecuteMsg as SwapExecuteMsg, Route, SmartSwapExactAssetIn, Swap, SwapExactAssetIn,
        SwapExactAssetOut, SwapOperation,
    },
};
use skip_api_entry_point::{error::ContractError, state::SWAP_VENUE_MAP};
use test_case::test_case;

/*
Test Cases:

Expect Response
    // Swap Exact Coin In
    - User Swap Exact Coin In With No Affiliates
    - User Swap Exact Coin In With Single Affiliate
    - User Swap Exact Coin In With Multiple Affiliates
    - User Swap Exact Coin In With Zero Fee Affiliate
    - User Swap Exact Cw20 Asset In With Single Affiliate

    // Swap Exact Coin Out
    - User Swap Exact Coin Out With No Affiliates
    - User Swap Exact Coin Out With Single Affiliate
    - User Swap Exact Coin Out With Multiple Affiliates
    - User Swap Exact Coin Out With Zero Fee Affiliate
    - User Swap Exact Coin Out With Refund Amount Zero (Ensure No Refund Message Included)
    - User Swap Exact Cw20 Asset Out With Single Affiliate

Expect Error
    // Swap Exact Coin In
    - User Swap Exact Coin In First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom
    - User Swap Exact Coin In Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom
    - User Swap Exact Coin In Empty Swap Operations

    // Swap Exact Coin Out
    - User Swap Exact Coin Out First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom
    - User Swap Exact Coin Out Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom
    - User Swap Exact Coin Out Empty Swap Operations
    - User Swap Exact Coin Out With No Refund Address
    - User Swap Exact Coin Out Where Coin In Denom Is Not The Same As Remaining Coin Received Denom
    - User Swap Exact Coin Out Where Coin In Amount More Than Remaining Coin Received Amount
    - User Swap Exact Asset Out Where Asset In Amount More Than Remaining Asset Received Amount

    // General
    - Unauthorized Caller

 */

// Define test parameters
struct Params {
    caller: String,
    user_swap: Swap,
    remaining_asset: Asset,
    min_asset: Asset,
    affiliates: Vec<Affiliate>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap_and_action
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
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
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin In With No Affiliates")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin In With Single Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name_2".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        remaining_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::new(1_000_000),
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "neutron123".to_string(), 
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: "swap_venue_adapter_2".to_string(),
                        amount: Uint128::new(1_000_000),
                        msg: to_json_binary(&SwapExecuteMsg::Swap {
                            operations: vec![
                                SwapOperation {
                                    pool: "pool".to_string(),
                                    denom_in: "neutron123".to_string(),
                                    denom_out: "os".to_string(),
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
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(100_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Cw20 Asset In With Single Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![
            Affiliate {
                address: "affiliate_1".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
            Affiliate {
                address: "affiliate_2".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate_1".to_string(),
                    amount: vec![Coin::new(100_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate_2".to_string(),
                    amount: vec![Coin::new(100_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin In With Multiple Affiliates")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(0),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(1_000_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin In With Zero Fee Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "refund_address".to_string(),
                    amount: vec![Coin::new(500_000, "un")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(500_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin Out With No Affiliates")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![
            Affiliate {
                address: "affiliate".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "refund_address".to_string(),
                    amount: vec![Coin::new(500_000, "un")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(500_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate".to_string(),
                    amount: vec![Coin::new(50_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin Out With Single Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name_2".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "neutron987".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::new(1_000_000),
        }),
        min_asset: Asset::Cw20(Cw20Coin {
            address: "neutron987".to_string(),
            amount: Uint128::new(1_000_000),
        }),
        affiliates: vec![Affiliate {
            address: "affiliate".to_string(),
            basis_points_fee: Uint128::new(1000),
        }],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "neutron123".to_string(), 
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "refund_address".to_string(),
                        amount: Uint128::new(500_000),
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
                    contract_addr: "neutron123".to_string(), 
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: "swap_venue_adapter_2".to_string(),
                        amount: Uint128::new(500_000),
                        msg: to_json_binary(&SwapExecuteMsg::Swap {
                            operations: vec![
                                SwapOperation {
                                    pool: "pool".to_string(),
                                    denom_in: "neutron123".to_string(),
                                    denom_out: "neutron987".to_string(),
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
                msg: WasmMsg::Execute {
                    contract_addr: "neutron987".to_string(), 
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "affiliate".to_string(),
                        amount: Uint128::new(100_000),
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
    "User Swap Exact Cw20 Asset Out With Single Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![
            Affiliate {
                address: "affiliate_1".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
            Affiliate {
                address: "affiliate_2".to_string(),
                basis_points_fee: Uint128::new(1000),
            },
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "refund_address".to_string(),
                    amount: vec![Coin::new(500_000, "un")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(500_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate_1".to_string(),
                    amount: vec![Coin::new(50_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "affiliate_2".to_string(),
                    amount: vec![Coin::new(50_000, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin Out With Multiple Affiliates")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![
            Affiliate {
                address: "affiliate".to_string(),
                basis_points_fee: Uint128::new(0),
            },
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "refund_address".to_string(),
                    amount: vec![Coin::new(500_000, "un")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_venue_adapter".to_string(), 
                    msg: to_json_binary(&SwapExecuteMsg::Swap {
                        operations: vec![
                            SwapOperation {
                                pool: "pool".to_string(),
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(500_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin Out With Zero Fee Affiliate")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(500_000, "un")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
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
                                denom_in: "un".to_string(),
                                denom_out: "os".to_string(),
                                interface: None,
                            }
                        ],
                    }).unwrap(),
                    funds: vec![Coin::new(500_000, "un")], 
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "User Swap Exact Coin Out With Refund Amount Zero")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "ua".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "uo")),
        min_asset: Asset::Native(Coin::new(100_000, "ua")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsAssetInDenomMismatch)),
    };
    "User Swap Exact Coin In First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "uo".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "uo")),
        min_asset: Asset::Native(Coin::new(100_000, "ua")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsAssetOutDenomMismatch)),
    };
    "User Swap Exact Coin In Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![],
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsEmpty)),
    };
    "User Swap Exact Coin In Empty Swap Operations - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "ua".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "uo")),
        min_asset: Asset::Native(Coin::new(100_000, "ua")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsAssetInDenomMismatch)),
    };
    "User Swap Exact Coin Out First Swap Operation Denom In Is Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "os".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "os")),
        min_asset: Asset::Native(Coin::new(100_000, "ua")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsAssetOutDenomMismatch)),
    };
    "User Swap Exact Coin Out Last Swap Operation Denom Out Is Not The Same As Min Coin Out Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![],
                refund_address: Some("refund_address".to_string()),
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(SwapOperationsEmpty)),
    };
    "User Swap Exact Coin Out Empty Swap Operations - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: None,
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::NoRefundAddress),
    };
    "User Swap Exact Coin Out With No Refund Address - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "ua".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "ua")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::UserSwapAssetInDenomMismatch),
    };
    "User Swap Exact Coin Out Where Coin In Denom Is Not The Same As Remaining Coin Received Denom - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Native(Coin::new(499_999, "un")),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "499999".to_string(),
            operand2: "500000".to_string(),
        }))),
    };
    "User Swap Exact Coin Out Where Coin In Amount More Than Remaining Coin Received Amount - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        user_swap: Swap::SwapExactAssetOut (
            SwapExactAssetOut{
                swap_venue_name: "swap_venue_name_2".to_string(),
                operations: vec![
                    SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }
                ],
                refund_address: Some("refund_address".to_string()),
            }
        ),
        remaining_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::new(499_999),
        }),
        min_asset: Asset::Native(Coin::new(500_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "499999".to_string(),
            operand2: "500000".to_string(),
        }))),
    };
    "User Swap Exact Asset Out Where Asset In Amount More Than Remaining Asset Received Amount - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        user_swap: Swap::SwapExactAssetIn (
            SwapExactAssetIn {
                swap_venue_name: "swap_venue_name".to_string(),
                operations: vec![],
            },
        ),
        remaining_asset: Asset::Native(Coin::new(1_000_000, "os")),
        min_asset: Asset::Native(Coin::new(1_000_000, "os")),
        affiliates: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
#[test_case(Params {
    caller: "entry_point".to_string(),
    user_swap: Swap::SmartSwapExactAssetIn(SmartSwapExactAssetIn {
        swap_venue_name: "swap_venue_name".to_string(),
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(250_000, "un")),
                operations: vec![SwapOperation {
                    pool: "pool".to_string(),
                    denom_in: "un".to_string(),
                    denom_out: "os".to_string(),
                    interface: None,
                }],
            },
            Route {
                offer_asset: Asset::Native(Coin::new(750_000, "un")),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "neutron123".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "pool_3".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    },
                ],
            },
        ],
    }),
    remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
    min_asset: Asset::Native(Coin::new(1_000_000, "os")),
    affiliates: vec![],
    expected_messages: vec![
        SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "swap_venue_adapter".to_string(),
                msg: to_json_binary(&SwapExecuteMsg::Swap {
                    operations: vec![SwapOperation {
                        pool: "pool".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    }],
                })
                .unwrap(),
                funds: vec![Coin::new(250_000, "un")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        },
        SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "swap_venue_adapter".to_string(),
                msg: to_json_binary(&SwapExecuteMsg::Swap {
                    operations: vec![
                        SwapOperation {
                            pool: "pool_2".to_string(),
                            denom_in: "un".to_string(),
                            denom_out: "neutron123".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "pool_3".to_string(),
                            denom_in: "neutron123".to_string(),
                            denom_out: "os".to_string(),
                            interface: None,
                        },
                    ],
                })
                .unwrap(),
                funds: vec![Coin::new(750_000, "un")],
            }
            .into(),
            gas_limit: None,
            reply_on: Never,
        },
    ],
    expected_error: None,
}; "SmartSwapExactAssetIn")]
#[test_case(Params {
    caller: "entry_point".to_string(),
    user_swap: Swap::SmartSwapExactAssetIn(SmartSwapExactAssetIn {
        swap_venue_name: "swap_venue_name".to_string(),
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(250_000, "un")),
                operations: vec![SwapOperation {
                    pool: "pool".to_string(),
                    denom_in: "un".to_string(),
                    denom_out: "os".to_string(),
                    interface: None,
                }],
            },
            Route {
                offer_asset: Asset::Native(Coin::new(750_000, "un")),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "un".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "pool_3".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "os".to_string(),
                        interface: None,
                    },
                ],
            },
        ],
    }),
    remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
    min_asset: Asset::Native(Coin::new(1_000_000, "os")),
    affiliates: vec![],
    expected_messages: vec![],
    expected_error: Some(ContractError::Skip(SwapOperationsAssetInDenomMismatch)),
}; "SmartSwapExactAssetIn With Mismatched Denom In - Expect Error")]
#[test_case(Params {
    caller: "entry_point".to_string(),
    user_swap: Swap::SmartSwapExactAssetIn(SmartSwapExactAssetIn {
        swap_venue_name: "swap_venue_name".to_string(),
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(250_000, "un")),
                operations: vec![SwapOperation {
                    pool: "pool".to_string(),
                    denom_in: "un".to_string(),
                    denom_out: "os".to_string(),
                    interface: None,
                }],
            },
            Route {
                offer_asset: Asset::Native(Coin::new(750_000, "un")),
                operations: vec![
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "un".to_string(),
                        denom_out: "neutron123".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "pool_3".to_string(),
                        denom_in: "neutron123".to_string(),
                        denom_out: "oa".to_string(),
                        interface: None,
                    },
                ],
            },
        ],
    }),
    remaining_asset: Asset::Native(Coin::new(1_000_000, "un")),
    min_asset: Asset::Native(Coin::new(1_000_000, "os")),
    affiliates: vec![],
    expected_messages: vec![],
    expected_error: Some(ContractError::Skip(SwapOperationsAssetOutDenomMismatch)),
}; "SmartSwapExactAssetIn With Mismatched Denom Out - Expect Error")]
fn test_execute_user_swap(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "os"), Coin::new(1_000_000, "un")],
    )]);

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "swap_venue_adapter" {
                    SystemResult::Ok(ContractResult::Ok(
                        to_json_binary(&Asset::Native(Coin::new(500_000, "un"))).unwrap(),
                    ))
                } else {
                    SystemResult::Ok(ContractResult::Ok(
                        to_json_binary(&Asset::Cw20(Cw20Coin {
                            address: "neutron123".to_string(),
                            amount: Uint128::new(500_000),
                        }))
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

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, &[]);

    // Store the swap venue adapter contract address in the swap venue map
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

    // Call execute_swap_and_action with the given test case params
    let res = skip_api_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::UserSwap {
            swap: params.user_swap,
            remaining_asset: params.remaining_asset,
            min_asset: params.min_asset,
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
