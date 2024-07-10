use std::collections::VecDeque;

#[allow(unused_imports)]
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Binary, Coin,
    ReplyOn::Never,
    ReplyOn::Success,
    SubMsg, WasmMsg,
};
use cosmwasm_std::{Reply, SubMsgResponse, SubMsgResult};
#[allow(unused_imports)]
use pryzm_std::types::{
    cosmos::base::v1beta1::Coin as CosmosCoin,
    pryzm::amm::v1::{MsgBatchSwap, MsgBatchSwapResponse, SwapStep, SwapType},
    pryzm::icstaking::v1::{MsgStake, MsgStakeResponse},
};
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
    - One-Step Left
    - Many Steps Left

 */

// Define test parameters
struct Params {
    swapper: String,
    reply_id: u64,
    swap_steps: Vec<SwapExecutionStep>,
    response: Binary,
    expected_messages: Vec<SubMsg>,
    expected_error_string: String,
    expected_stored_swapper: String,
    expected_stored_steps: Vec<SwapExecutionStep>,
}

// Test execute_swap
#[test_case(
    Params {
        swapper: "entry_point".to_string(),
        reply_id: STAKE_REPLY_ID,
        swap_steps: vec![
            SwapExecutionStep::Stake {
                host_chain_id: "uatom".to_string(),
                transfer_channel: "channel-0".to_string()
            },
            SwapExecutionStep::Swap {
                swap_steps: vec![
                    SwapStep {
                        pool_id: 4,
                        token_in: "c:uatom".to_string(),
                        token_out: "p:uatom:30Sep2024".to_string(),
                        amount: None,
                    },
                ],
            },
        ],
        response: MsgStakeResponse {
            c_amount: Some(CosmosCoin {amount: "1800".to_string(), denom: "c:uatom".to_string()}),
            fee: None,
        }.into(),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: MsgBatchSwap {
                    creator: "swap_contract_address".to_string(),
                    swap_type: SwapType::GivenIn.into(),
                    max_amounts_in: vec![CosmosCoin{amount: "1800".to_string(), denom: "c:uatom".to_string()}],
                    min_amounts_out: vec![CosmosCoin{amount: "1".to_string(), denom: "p:uatom:30Sep2024".to_string()}],
                    steps: vec![
                        SwapStep {
                            pool_id: 4,
                            token_in: "c:uatom".to_string(),
                            token_out: "p:uatom:30Sep2024".to_string(),
                            amount: Some("1800".to_string()),
                        }
                    ],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "p:uatom:30Sep2024".to_string(),
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
"One Step Left")]
#[test_case(
    Params {
        swapper: "entry_point".to_string(),
        reply_id: BATCH_SWAP_REPLY_ID,
        swap_steps: vec![
            SwapExecutionStep::Swap {
                swap_steps: vec![
                    SwapStep {
                        pool_id: 1,
                        token_in: "ibc/uosmo".to_string(),
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
            },
            SwapExecutionStep::Stake {
                host_chain_id: "uatom".to_string(),
                transfer_channel: "channel-0".to_string()
            },
            SwapExecutionStep::Swap {
                swap_steps: vec![
                    SwapStep {
                        pool_id: 1,
                        token_in: "c:uatom".to_string(),
                        token_out: "lp:0:uatomlpt".to_string(),
                        amount: None,
                    }
                ],
            },
        ],
        response: MsgBatchSwapResponse {
            amounts_out: vec![CosmosCoin {amount: "100".to_string(), denom: "ibc/uatom".to_string()}],
            amounts_in: vec![],
            join_exit_protocol_fee: vec![],
            swap_fee: vec![],
            swap_protocol_fee: vec![],
        }.into(),
        expected_messages: vec![
            SubMsg {
                id: STAKE_REPLY_ID,
                msg: MsgStake {
                    creator: "swap_contract_address".to_string(),
                    host_chain: "uatom".to_string(),
                    transfer_channel: "channel-0".to_string(),
                    amount: "100".to_string(),
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
                        pool_id: 1,
                        token_in: "c:uatom".to_string(),
                        token_out: "lp:0:uatomlpt".to_string(),
                        amount: None,
                    }
                ],
            },
        ],
    };
"Multi Step Left")]
fn test_execute_reply(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Fill the storage
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;
    IN_PROGRESS_SWAP_OPERATIONS.save(deps.as_mut().storage, &VecDeque::from(params.swap_steps))?;
    IN_PROGRESS_SWAP_SENDER.save(
        deps.as_mut().storage,
        &Addr::unchecked(params.swapper.as_str()),
    )?;

    // Call execute_swap with the given test parameters
    let res = contract::reply(
        deps.as_mut(),
        env,
        Reply {
            id: params.reply_id,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: Some(params.response),
                events: vec![],
            }),
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
