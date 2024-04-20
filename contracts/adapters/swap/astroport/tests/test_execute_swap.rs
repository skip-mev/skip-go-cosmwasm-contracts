use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin,
    ReplyOn::Never,
    SubMsg, WasmMsg,
};
use skip::{
    asset::Asset,
    swap::{ExecuteMsg, Route, SwapOperation},
};
use skip_api_swap_adapter_astroport::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Swap Operation
    - Multiple Swap Operations
    - No Swap Operations (This is prevented in the entry point contract; and will not add any swap messages to the response)

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - No Coin Sent
    - More Than One Coin Sent

 */

// Define test parameters
struct Params {
    caller: String,
    info_funds: Vec<Coin>,
    routes: Vec<Route>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(100, "os")),
                operations: vec![
                    SwapOperation {
                        pool: "pool_1".to_string(),
                        denom_in: "os".to_string(),
                        denom_out: "ua".to_string(),
                        interface: None,
                    }
                ],
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::AstroportPoolSwap {
                        offer_asset: Some(Asset::Native(Coin::new(100, "os"))),
                        operation: SwapOperation {
                            pool: "pool_1".to_string(),
                            denom_in: "os".to_string(),
                            denom_out: "ua".to_string(),
                            interface: None,
                        }
                    })?,
                    funds: vec![],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "ua".to_string(),
                        swapper: Addr::unchecked("entry_point"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "One Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(100, "os")),
                operations: vec![
                    SwapOperation {
                        pool: "pool_1".to_string(),
                        denom_in: "os".to_string(),
                        denom_out: "ua".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "pool_2".to_string(),
                        denom_in: "ua".to_string(),
                        denom_out: "un".to_string(),
                        interface: None,
                    }
                ],
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::AstroportPoolSwap {
                        offer_asset: Some(Asset::Native(Coin::new(100, "os"))),
                        operation: SwapOperation {
                            pool: "pool_1".to_string(),
                            denom_in: "os".to_string(),
                            denom_out: "ua".to_string(),
                            interface: None,
                        }
                    })?,
                    funds: vec![],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::AstroportPoolSwap {
                        offer_asset: None,
                        operation: SwapOperation {
                            pool: "pool_2".to_string(),
                            denom_in: "ua".to_string(),
                            denom_out: "un".to_string(),
                            interface: None,
                        }
                    })?,
                    funds: vec![],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "un".to_string(),
                        swapper: Addr::unchecked("entry_point"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Multiple Swap Operations")]
    #[test_case(
        Params {
            caller: "entry_point".to_string(),
            info_funds: vec![Coin::new(100, "os")],
            routes: vec![
                Route {
                    offer_asset: Asset::Native(Coin::new(25, "os")),
                    operations: vec![
                        SwapOperation {
                            pool: "pool_1".to_string(),
                            denom_in: "os".to_string(),
                            denom_out: "ua".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "pool_2".to_string(),
                            denom_in: "ua".to_string(),
                            denom_out: "un".to_string(),
                            interface: None,
                        }
                    ],
                },
                Route {
                    offer_asset: Asset::Native(Coin::new(75, "os")),
                    operations: vec![
                        SwapOperation {
                            pool: "pool_3".to_string(),
                            denom_in: "os".to_string(),
                            denom_out: "un".to_string(),
                            interface: None,
                        },
                    ],
                }                
            ],
            expected_messages: vec![
                SubMsg {
                    id: 0,
                    msg: WasmMsg::Execute {
                        contract_addr: "swap_contract_address".to_string(),
                        msg: to_json_binary(&ExecuteMsg::AstroportPoolSwap {
                            offer_asset: Some(Asset::Native(Coin::new(25, "os"))),
                            operation: SwapOperation {
                                pool: "pool_1".to_string(),
                                denom_in: "os".to_string(),
                                denom_out: "ua".to_string(),
                                interface: None,
                            }
                        })?,
                        funds: vec![],
                    }.into(),
                    gas_limit: None,
                    reply_on: Never,
                },
                SubMsg {
                    id: 0,
                    msg: WasmMsg::Execute {
                        contract_addr: "swap_contract_address".to_string(),
                        msg: to_json_binary(&ExecuteMsg::AstroportPoolSwap {
                            offer_asset: None,
                            operation: SwapOperation {
                                pool: "pool_2".to_string(),
                                denom_in: "ua".to_string(),
                                denom_out: "un".to_string(),
                                interface: None,
                            }
                        })?,
                        funds: vec![],
                    }.into(),
                    gas_limit: None,
                    reply_on: Never,
                },
                SubMsg {
                    id: 0,
                    msg: WasmMsg::Execute {
                        contract_addr: "swap_contract_address".to_string(),
                        msg: to_json_binary(&ExecuteMsg::AstroportPoolSwap {
                            offer_asset: Some(Asset::Native(Coin::new(75, "os"))),
                            operation: SwapOperation {
                                pool: "pool_3".to_string(),
                                denom_in: "os".to_string(),
                                denom_out: "un".to_string(),
                                interface: None,
                            }
                        })?,
                        funds: vec![],
                    }.into(),
                    gas_limit: None,
                    reply_on: Never,
                },                
                SubMsg {
                    id: 0,
                    msg: WasmMsg::Execute {
                        contract_addr: "swap_contract_address".to_string(),
                        msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                            return_denom: "un".to_string(),
                            swapper: Addr::unchecked("entry_point"),
                        })?,
                        funds: vec![],
                    }
                    .into(),
                    gas_limit: None,
                    reply_on: Never,
                },
            ],
            expected_error: None,
        };
        "Multiple Routes")]    
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(100, "os")),
                operations: vec![],
            }
        ],
        expected_messages: vec![],
        expected_error: Some(ContractError::SwapOperationsEmpty),
    };
    "No Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        routes: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(cw_utils::PaymentError::NoFunds{})),
    };
    "No Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "un"),
            Coin::new(100, "os"),
        ],
        routes: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(cw_utils::PaymentError::MultipleDenoms{})),
    };
    "More Than One Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![
            Coin::new(100, "un"),
        ],
        routes: vec![
            Route {
                offer_asset: Asset::Native(Coin::new(100, "un")),
                operations: vec![],
            }
        ],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, info_funds);

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;

    // Call execute_swap with the given test parameters
    let res = skip_api_swap_adapter_astroport::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Swap {
            routes: params.routes,
        },
    );

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the messages are correct
            assert_eq!(res.messages, params.expected_messages);
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

    Ok(())
}
