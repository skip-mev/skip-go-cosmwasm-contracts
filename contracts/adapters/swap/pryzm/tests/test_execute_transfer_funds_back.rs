use cosmwasm_std::{
    Addr,
    Coin, SubMsg, testing::{mock_dependencies_with_balances, mock_env, mock_info},
};
use test_case::test_case;

use skip::swap::ExecuteMsg;
use skip_api_swap_adapter_pryzm::error::{ContractError, ContractResult};

/*
Test Cases:

Expect Success
    - One Coin Balance
    - Multiple Coin Balance
    - No Coin Balance (This will fail at the bank module if attempted)

Expect Error
    - Unauthorized Caller (Only contract itself can call this function)
 */

// Define test parameters
struct Params {
    caller: String,
    contract_balance: Vec<Coin>,
    return_denom: String,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_transfer_funds_back
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![Coin::new(100, "os")],
        return_denom: "os".to_string(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![Coin::new(100, "os")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Transfers One Coin Balance")]
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![
            Coin::new(100, "os"),
            Coin::new(100, "uatom"),
        ],
        return_denom: "os".to_string(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![
                        Coin::new(100, "os"),
                        Coin::new(100, "uatom")
                    ],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Transfers Multiple Coin Balance")]
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![],
        return_denom: "os".to_string(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Transfers No Coin Balance")]
#[test_case(
    Params {
        caller: "random".to_string(),
        contract_balance: vec![],
        return_denom: "os".to_string(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: Some(ContractError::Skip(SkipError::Unauthorized)),
    };
    "Unauthorized Caller")]
fn test_execute_transfer_funds_back(params: Params) -> ContractResult<()> {
    // Convert params contract balance to a slice
    let contract_balance: &[Coin] = &params.contract_balance;

    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[("swap_contract_address", contract_balance)]);

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Create mock info
    let info = mock_info(&params.caller, &[]);

    // Call execute_swap with the given test parameters
    let res = skip_api_swap_adapter_pryzm::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::TransferFundsBack {
            return_denom: params.return_denom,
            swapper: Addr::unchecked("swapper"),
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
