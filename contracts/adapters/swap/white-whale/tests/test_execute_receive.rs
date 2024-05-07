use core::panic;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin, ContractResult as SystemContractResult, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg};
use cw_utils::PaymentError::NonPayable;
use skip::{
    asset::Asset,
    error::SkipError::Payment,
    swap::{ExecuteMsg, SwapOperation},
};
use skip_api_swap_adapter_white_whale::{
    error::{ContractError, ContractResult},
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Swap Operation - Cw20 In
    - One Swap Operation - Cw20 In And Out

Expect Error
    - Coin sent with cw20

 */

// Define test parameters
struct Params {
    caller: String,
    info_funds: Vec<Coin>,
    sent_asset: Asset,
    swap_operations: Vec<SwapOperation>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(100u128)
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "neutron123".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::WhiteWhalePoolSwap {
                        operation: SwapOperation {
                            pool: "pool_1".to_string(),
                            denom_in: "neutron123".to_string(),
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
    "One Swap Operation - Cw20 In")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(100u128)
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "neutron123".to_string(),
                denom_out: "neutron987".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::WhiteWhalePoolSwap {
                        operation: SwapOperation {
                            pool: "pool_1".to_string(),
                            denom_in: "neutron123".to_string(),
                            denom_out: "neutron987".to_string(),
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
                        return_denom: "neutron987".to_string(),
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
    "One Swap Operation - Cw20 In And Out")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "un"),
        ],
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "neutron123".to_string(),
            amount: Uint128::from(100u128)
        }),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Payment(NonPayable{}))),
    };
    "Coin sent with cw20 - Expect Error")]
fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "neutron123" {
                    SystemResult::Ok(SystemContractResult::Ok(
                        to_json_binary(&BalanceResponse {
                            balance: Uint128::from(100u128),
                        })
                        .unwrap(),
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
    let info = mock_info(params.sent_asset.denom(), info_funds);

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;

    // Call execute_swap with the given test parameters
    let res = skip_api_swap_adapter_white_whale::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: params.caller,
            amount: params.sent_asset.amount(),
            msg: to_json_binary(&ExecuteMsg::Swap {
                operations: params.swap_operations,
            })
            .unwrap(),
        }),
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
