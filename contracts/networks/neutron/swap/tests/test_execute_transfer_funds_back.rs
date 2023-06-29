use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    Addr, BankMsg, Coin,
    ReplyOn::Never,
    SubMsg,
};
use skip::swap::ExecuteMsg;
use skip_swap_neutron_astroport_swap::error::{ContractError, ContractResult};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Coin Balance
    - Multiple Coin Balance
    - No Coin Balance (This will fail at the bank module if attempted)
 */

// Define test parameters
struct Params {
    contract_balance: Vec<Coin>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![Coin::new(100, "uosmo")],
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
        contract_balance: vec![
            Coin::new(100, "uosmo"),
            Coin::new(100, "uatom"),
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![
                        Coin::new(100, "uosmo"),
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
        contract_balance: vec![],
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
fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Convert params contract balance to a slice
    let contract_balance: &[Coin] = &params.contract_balance;

    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[("swap_contract_address", contract_balance)]);

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Create mock info
    let info = mock_info("swap_contract_address", &[]);

    // Call execute_swap with the given test parameters
    let res = skip_swap_neutron_astroport_swap::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::TransferFundsBack {
            caller: Addr::unchecked("swapper"),
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
