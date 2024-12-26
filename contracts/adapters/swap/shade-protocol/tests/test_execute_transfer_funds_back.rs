use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, BankMsg, Coin, ContractInfo, ContractResult as SystemContractResult,
    QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw20::BalanceResponse;
use skip_go_swap_adapter_shade_protocol::{
    error::{ContractError, ContractResult},
    msg::ExecuteMsg,
    state::{ENTRY_POINT_CONTRACT, REGISTERED_TOKENS, VIEWING_KEY},
};
use test_case::test_case;

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
        contract_balance: vec![Coin::new(100, "secret123")],
        return_denom: "secret123".to_string(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![Coin::new(100, "secret123")],
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
            Coin::new(100, "secret123"),
            Coin::new(100, "secret456"),
        ],
        return_denom: "secret123".to_string(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "swapper".to_string(),
                    amount: vec![
                        Coin::new(100, "secret123"),
                        Coin::new(100, "secret456")
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
        return_denom: "secret123".to_string(),
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
        return_denom: "secret123".to_string(),
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
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller")]
fn test_execute_transfer_funds_back(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    let contract_balance = params.contract_balance.clone();

    // Mock contract balance querys
    let wasm_handler = move |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                match contract_balance
                    .iter()
                    .find(|coin| coin.denom == *contract_addr)
                {
                    Some(coin) => SystemResult::Ok(SystemContractResult::Ok(
                        to_binary(&BalanceResponse {
                            balance: coin.amount.u128().into(),
                        })
                        .unwrap(),
                    )),
                    None => panic!("Unsupported contract balance query: {:?}", query),
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
    env.contract.code_hash = "code_hash".to_string();

    // Create mock info
    let info = mock_info(&params.caller.clone(), &[]);

    VIEWING_KEY.save(&mut deps.storage, &"viewing_key".to_string())?;
    ENTRY_POINT_CONTRACT.save(
        &mut deps.storage,
        &ContractInfo {
            address: Addr::unchecked("entry_point_contract"),
            code_hash: "code_hash".to_string(),
        },
    )?;
    for coin in params.contract_balance.iter() {
        REGISTERED_TOKENS.save(
            &mut deps.storage,
            Addr::unchecked(coin.denom.clone()),
            &ContractInfo {
                address: Addr::unchecked(coin.denom.clone()),
                code_hash: "code_hash".to_string(),
            },
        )?;
    }

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_shade_protocol::contract::execute(
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
