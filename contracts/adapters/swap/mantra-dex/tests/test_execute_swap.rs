use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin, Decimal,
    ReplyOn::Never,
    SubMsg, WasmMsg,
};
use skip::swap::{ExecuteMsg, SwapOperation};
use skip_go_swap_adapter_mantra_dex::error::{ContractError, ContractResult};
use skip_go_swap_adapter_mantra_dex::state::{
    ENTRY_POINT_CONTRACT_ADDRESS, MANTRA_DEX_POOL_MANAGER_ADDRESS,
};
use test_case::test_case;

use skip_go_swap_adapter_mantra_dex::pool_manager::{
    ExecuteMsg as MantraPoolManagerExecuteMsg, SwapOperation as MantraSwapOperation,
};

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
                    contract_addr: "mantra_pool_manager".to_string(),
                    msg: to_json_binary(&MantraPoolManagerExecuteMsg::ExecuteSwapOperations {
                        operations: vec![MantraSwapOperation::MantraSwap {
                            pool_identifier: "pool_1".to_string(),
                            token_in_denom: "os".to_string(),
                            token_out_denom: "ua".to_string(),
                        }],
                        minimum_receive: None,
                        receiver: Some("entry_point".to_string()),
                        max_spread: Some(Decimal::percent(50)),
                    })?,
                    funds: vec![Coin::new(100, "os")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            }
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
                    contract_addr: "mantra_pool_manager".to_string(),
                    msg: to_json_binary(&MantraPoolManagerExecuteMsg::ExecuteSwapOperations {
                        operations: vec![MantraSwapOperation::MantraSwap {
                            pool_identifier: "pool_1".to_string(),
                            token_in_denom: "os".to_string(),
                            token_out_denom: "ua".to_string(),
                        },
                        MantraSwapOperation::MantraSwap {
                            pool_identifier: "pool_2".to_string(),
                            token_in_denom: "ua".to_string(),
                            token_out_denom: "un".to_string(),
                        }
                        ],
                        minimum_receive: None,
                        receiver: Some("entry_point".to_string()),
                        max_spread: Some(Decimal::percent(50)),
                    })?,
                    funds: vec![Coin::new(100, "os")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            }
        ],
        expected_error: None,
    };
    "Multiple Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::SwapOperationsEmpty),
    };
    "No Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        swap_operations: vec![],
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
        swap_operations: vec![],
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
        swap_operations: vec![],
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
    MANTRA_DEX_POOL_MANAGER_ADDRESS.save(
        deps.as_mut().storage,
        &Addr::unchecked("mantra_pool_manager"),
    )?;

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_mantra_dex::contract::execute(
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
