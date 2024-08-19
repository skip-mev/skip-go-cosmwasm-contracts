#[allow(unused_imports)]
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin,
    ReplyOn::Never,
    ReplyOn::Success,
    SubMsg, WasmMsg,
};
#[allow(unused_imports)]
use pryzm_std::types::{
    cosmos::base::v1beta1::Coin as CosmosCoin,
    pryzm::amm::v1::{MsgBatchSwap, SwapStep, SwapType},
    pryzm::icstaking::v1::MsgStake,
};
use std::collections::VecDeque;
use test_case::test_case;

#[allow(unused_imports)]
use skip::swap::{ExecuteMsg, SwapOperation};

use skip_go_swap_adapter_pryzm::execution::SwapExecutionStep;
use skip_go_swap_adapter_pryzm::state::{IN_PROGRESS_SWAP_OPERATIONS, IN_PROGRESS_SWAP_SENDER};
#[allow(unused_imports)]
use skip_go_swap_adapter_pryzm::{
    contract, error::ContractResult, reply_id::BATCH_SWAP_REPLY_ID, reply_id::STAKE_REPLY_ID,
    state::ENTRY_POINT_CONTRACT_ADDRESS,
};
/*
Test Cases:

Expect Success
    - One Swap Operation
    - Multiple Swap Operations
    - No Swap Operations (This is prevented in the entry point contract)

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
    expected_stored_swapper: String,
    expected_stored_steps: Vec<SwapExecutionStep>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "pr")],
        swap_operations: vec![
            SwapOperation {
                pool: "amm:1".to_string(),
                denom_in: "pr".to_string(),
                denom_out: "ibc/uusdc".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: MsgBatchSwap {
                    creator: "swap_contract_address".to_string(),
                    swap_type: SwapType::GivenIn.into(),
                    max_amounts_in: vec![CosmosCoin {amount: "100".to_string(), denom: "pr".to_string()}],
                    min_amounts_out: vec![CosmosCoin {amount: "1".to_string(), denom: "ibc/uusdc".to_string()}],
                    steps: vec![
                        SwapStep {
                            pool_id: 1,
                            token_in: "pr".to_string(),
                            token_out: "ibc/uusdc".to_string(),
                            amount: Some("100".to_string()),
                        }
                    ]
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "ibc/uusdc".to_string(),
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
        expected_stored_swapper: "".to_string(),
        expected_stored_steps: vec![],
    };
"One Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(2000, "pr")],
        swap_operations: vec![
            SwapOperation {
                pool: "amm:1".to_string(),
                denom_in: "pr".to_string(),
                denom_out: "ibc/uusdc".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "amm:2".to_string(),
                denom_in: "ibc/uusdc".to_string(),
                denom_out: "ibc/uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "ibc/uatom".to_string(),
                denom_out: "c:uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "amm:3".to_string(),
                denom_in: "c:uatom".to_string(),
                denom_out: "p:uatom:30Sep2024".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: BATCH_SWAP_REPLY_ID,
                msg: MsgBatchSwap {
                    creator: "swap_contract_address".to_string(),
                    swap_type: SwapType::GivenIn.into(),
                    max_amounts_in: vec![CosmosCoin {
                        amount: "2000".to_string(),
                        denom: "pr".to_string(),
                    }],
                    min_amounts_out: vec![CosmosCoin {
                        amount: "1".to_string(),
                        denom: "ibc/uatom".to_string(),
                    }],
                    steps: vec![
                        SwapStep {
                            pool_id: 1,
                            token_in: "pr".to_string(),
                            token_out: "ibc/uusdc".to_string(),
                            amount: Some("2000".to_string()),
                        },
                        SwapStep {
                            pool_id: 2,
                            token_in: "ibc/uusdc".to_string(),
                            token_out: "ibc/uatom".to_string(),
                            amount: None,
                        },
                    ],
                }
                .into(),
                gas_limit: None,
                reply_on: Success,
            },
        ],
        expected_error_string: "".to_string(),
        expected_stored_swapper: "entry_point".to_string(),
        expected_stored_steps: vec![
            SwapExecutionStep::Swap {
                swap_steps: vec![
                    SwapStep {
                        pool_id: 1,
                        token_in: "pr".to_string(),
                        token_out: "ibc/uusdc".to_string(),
                        amount: None,
                    },
                    SwapStep {
                        pool_id: 2,
                        token_in: "ibc/uusdc".to_string(),
                        token_out: "ibc/uatom".to_string(),
                        amount: None,
                    },
                ],
            },
            SwapExecutionStep::Stake {
                host_chain_id: "uatom".to_string(),
                transfer_channel: "channel-0".to_string()
            },
            SwapExecutionStep::Swap {
                swap_steps: vec![
                    SwapStep {
                        pool_id: 3,
                        token_in: "c:uatom".to_string(),
                        token_out: "p:uatom:30Sep2024".to_string(),
                        amount: None,
                    },
                ],
            },
        ],
    };
"Multiple Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(2000, "pr")],
        swap_operations: vec![
            SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "pr".to_string(),
                denom_out: "c:pr".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "amm:4".to_string(),
                denom_in: "c:pr".to_string(),
                denom_out: "p:pr:30Sep2024".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: STAKE_REPLY_ID,
                msg: MsgStake {
                    creator: "swap_contract_address".to_string(),
                    host_chain: "uatom".to_string(),
                    transfer_channel: "channel-0".to_string(),
                    amount: "2000".to_string(),
                }
                .into(),
                gas_limit: None,
                reply_on: Success,
            },
        ],
        expected_error_string: "".to_string(),
        expected_stored_swapper: "entry_point".to_string(),
        expected_stored_steps: vec![
            SwapExecutionStep::Stake {
                host_chain_id: "uatom".to_string(),
                transfer_channel: "channel-0".to_string()
            },
            SwapExecutionStep::Swap {
                swap_steps: vec![
                    SwapStep {
                        pool_id: 4,
                        token_in: "c:pr".to_string(),
                        token_out: "p:pr:30Sep2024".to_string(),
                        amount: None,
                    },
                ],
            },
        ],
    };
"First Step Liquid Staking")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "pr")],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error_string: "swap_operations cannot be empty".to_string(),
        expected_stored_swapper: "".to_string(),
        expected_stored_steps: vec![],
    };
"No Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "pr".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "No funds sent".to_string(),
        expected_stored_swapper: "".to_string(),
        expected_stored_steps: vec![],
    };
    "No Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "pr"),
            Coin::new(100, "uatom"),
        ],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "pr".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "Sent more than one denomination".to_string(),
        expected_stored_swapper: "".to_string(),
        expected_stored_steps: vec![],
    };
    "More Than One Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "pr")],
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "pr".to_string(),
                denom_out: "ibc/uusdc".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "provided pool string is not a valid swap route".to_string(),
        expected_stored_swapper: "".to_string(),
        expected_stored_steps: vec![],
    };
    "Invalid Pool For Swap Operations - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![Coin::new(100, "pr")],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
        expected_stored_swapper: "".to_string(),
        expected_stored_steps: vec![],
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
    let res = contract::execute(
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

            if !params.expected_stored_steps.is_empty() {
                // Assert the stored steps are correct
                let stored_steps = IN_PROGRESS_SWAP_OPERATIONS.load(deps.as_ref().storage)?;
                assert_eq!(stored_steps, VecDeque::from(params.expected_stored_steps));
            } else {
                // Assert no steps are stored
                assert!(IN_PROGRESS_SWAP_OPERATIONS
                    .load(deps.as_ref().storage)
                    .is_err());
            }

            if !params.expected_stored_swapper.is_empty() {
                // Assert the stored swapper is correct
                let stored_swapper = IN_PROGRESS_SWAP_SENDER.load(deps.as_ref().storage)?;
                assert_eq!(
                    stored_swapper,
                    Addr::unchecked(params.expected_stored_swapper.as_str())
                );
            } else {
                // Assert no address is stored
                assert!(IN_PROGRESS_SWAP_SENDER.load(deps.as_ref().storage).is_err());
            }
        }
        Err(err) => {
            // Assert the test expected an error
            assert!(
                !params.expected_error_string.is_empty(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert!(err
                .to_string()
                .contains(params.expected_error_string.as_str()));
        }
    }

    Ok(())
}
