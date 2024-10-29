use astrovault::assets::pools::PoolInfoInput;
use astrovault::router::handle_msg::RouterReceiveMsg;
use astrovault::router::state::HopV2;
use astrovault::{assets::asset::AssetInfo, router::query_msg::RoutePoolType};
use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, WasmMsg, WasmQuery,
};
use skip::swap::{ExecuteMsg, SwapOperation};
use skip_go_swap_adapter_astrovault::{
    error::{ContractError, ContractResult},
    state::{ASTROVAULT_ROUTER_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Swap Operation
    - Multiple Swap Operations

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - No Coin Sent
    - More Than One Coin Sent
 */

// Define test parameters
struct Params {
    caller: String,
    info_funds: Vec<Coin>,
    swap_operations: Vec<SwapOperation>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "astrovault_router".to_string(),
                    msg: to_json_binary(&astrovault::router::handle_msg::ExecuteMsg::Receive(
                        cw20::Cw20ReceiveMsg {
                            sender: "swap_contract_address".to_string(),
                            amount: cosmwasm_std::Uint128::from(100u128),
                            msg: to_json_binary(&RouterReceiveMsg::RouteV2 {
                                hops: vec![
                                    HopV2::RatioHopInfo { pool:
                                        PoolInfoInput::Addr("pool_1".to_string()), from_asset_index: 0 }
                                ],
                                minimum_receive: None,
                                to: None,
                            })?,
                        }
                    ))?,
                    funds: vec![coin(100, "os")],
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
        swap_operations: vec![
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
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "astrovault_router".to_string(),
                    msg: to_json_binary(&astrovault::router::handle_msg::ExecuteMsg::Receive(
                        cw20::Cw20ReceiveMsg {
                            sender: "swap_contract_address".to_string(),
                            amount: cosmwasm_std::Uint128::from(100u128),
                            msg: to_json_binary(&RouterReceiveMsg::RouteV2 {
                                hops: vec![
                                    HopV2::RatioHopInfo { pool:
                                        PoolInfoInput::Addr("pool_1".to_string()), from_asset_index: 0 },
                                    HopV2::StandardHopInfo { pool:
                                            PoolInfoInput::Addr("pool_2".to_string()), from_asset_index: 0 },
                                ],
                                minimum_receive: None,
                                to: None,
                            })?,
                        }
                    ))?,
                    funds: vec![coin(100, "os")],
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
        info_funds: vec![],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
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
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(cw_utils::PaymentError::MultipleDenoms{})),
    };
    "More Than One Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![
            Coin::new(100, "os"),
        ],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]

fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();
    let swap_ops = params.swap_operations.clone();

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = move |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "astrovault_router" {
                    let mut mock_route_pool_type_query_response = vec![];
                    if !swap_ops.is_empty() {
                        mock_route_pool_type_query_response.push(RoutePoolType {
                            pool_addr: "pool_1".to_string(),
                            pool_type: "hybrid".to_string(),
                            pool_asset_infos: vec![
                                AssetInfo::NativeToken {
                                    denom: "os".to_string(),
                                },
                                AssetInfo::NativeToken {
                                    denom: "ua".to_string(),
                                },
                            ],
                        });
                    }
                    if swap_ops.len() > 1 {
                        mock_route_pool_type_query_response.push(RoutePoolType {
                            pool_addr: "pool_2".to_string(),
                            pool_type: "standard".to_string(),
                            pool_asset_infos: vec![
                                AssetInfo::NativeToken {
                                    denom: "ua".to_string(),
                                },
                                AssetInfo::NativeToken {
                                    denom: "un".to_string(),
                                },
                            ],
                        });
                    }

                    SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                        to_json_binary(&mock_route_pool_type_query_response).unwrap(),
                    ))
                } else {
                    panic!("Unsupported contract: {:?}", query);
                }
            }
            _ => panic!("Unsupported query: {:?}", query),
        }
    };

    // Update querier with mock wasm handler
    deps.querier.update_wasm(wasm_handler);

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, info_funds);

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;
    ASTROVAULT_ROUTER_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("astrovault_router"))?;

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_astrovault::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Swap {
            operations: params.swap_operations.clone(),
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
