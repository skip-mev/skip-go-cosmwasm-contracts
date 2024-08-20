use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    SubMsg,
};
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, ReplyOn::Never, WasmMsg};
use neutron_sdk::{
    proto_types::neutron::dex::{MsgMultiHopSwap, MultiHopRoute},
    stargate::aux::create_stargate_msg,
};
use skip::swap::{ExecuteMsg, SwapOperation};

use skip_go_swap_adapter_duality::{error::ContractResult, state::ENTRY_POINT_CONTRACT_ADDRESS};

use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Swap Operation
    - Multiple Swap Operations
    - No Swap Operations (This is prevented in the entry point contract; and will fail on Duality module if attempted)

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - No Coin Sent
    - More Than One Coin Sent
    - Invalid Pool ID Conversion For Swap Operations

 */

// Define test parameters
struct Params {
    caller: String,
    info_funds: Vec<Coin>,
    swap_operations: Vec<SwapOperation>,
    expected_messages: Vec<SubMsg>,
    expected_error_string: String,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: get_multi_hop_msg(MsgMultiHopSwap {
                    creator: "swap_contract_address".to_string(),
                    receiver: "swap_contract_address".to_string(),
                    routes: vec![MultiHopRoute {hops: vec![String::from("os"),String::from("uatom")]}],
                    amount_in: String::from("100"),
                    exit_limit_price: String::from("000000000000000000000000001"),
                    pick_best_route: true,
                })?,
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "uatom".to_string(),
                        swapper: Addr::unchecked("entry_point"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error_string: "".to_string(),
    };
"One Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "untrn".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: get_multi_hop_msg(MsgMultiHopSwap {
                    creator: "swap_contract_address".to_string(),
                    receiver: "swap_contract_address".to_string(),
                    routes: vec![MultiHopRoute {hops: vec![String::from("os"),String::from("uatom"), String::from("untrn")]}],
                    amount_in: String::from("100"),
                    exit_limit_price: String::from("000000000000000000000000001"),
                    pick_best_route: true,
                })?,
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "untrn".to_string(),
                        swapper: Addr::unchecked("entry_point"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error_string: "".to_string(),
    };
"Multiple Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: get_multi_hop_msg(MsgMultiHopSwap {
                    creator: "swap_contract_address".to_string(),
                    receiver: "swap_contract_address".to_string(),
                    routes: vec![MultiHopRoute {hops: vec![]}],
                    amount_in: String::from("100"),
                    exit_limit_price: String::from("000000000000000000000000001"),
                    pick_best_route: true,
                })?,
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "os".to_string(),
                        swapper: Addr::unchecked("entry_point"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error_string: "swap_operations cannot be empty".to_string(),
    };
"No Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "No funds sent".to_string(),
    };
    "No Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "os"),
            Coin::new(100, "uatom"),
        ],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "Sent more than one denomination".to_string(),
    };
    "More Than One Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![
            Coin::new(100, "untrn"),
            Coin::new(100, "os"),
        ],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
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
    let res = skip_go_swap_adapter_duality::contract::execute(
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
                params.expected_error_string.is_empty(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error_string
            );

            // Assert the messages are correct
            assert_eq!(res.messages, params.expected_messages);
        }
        Err(err) => {
            // Assert the test expected an error
            assert!(
                !params.expected_error_string.is_empty(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert_eq!(err.to_string(), params.expected_error_string);
        }
    }

    Ok(())
}

fn get_multi_hop_msg(msg: MsgMultiHopSwap) -> ContractResult<CosmosMsg> {
    Ok(create_stargate_msg("/neutron.dex.MsgMultiHopSwap", msg))
}
