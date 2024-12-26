use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Coin, ContractInfo,
    ReplyOn::Never,
    SubMsg, WasmMsg,
};
use cw20::Cw20Coin;
use secret_skip::{
    asset::Asset,
    snip20::{self, Snip20ReceiveMsg},
    swap::SwapOperation,
};
use skip_go_swap_adapter_shade_protocol::{
    error::{ContractError, ContractResult},
    msg::{ExecuteMsg, Snip20HookMsg},
    shade_swap_router_msg as shade_router,
    state::{ENTRY_POINT_CONTRACT, REGISTERED_TOKENS, SHADE_POOL_CODE_HASH, SHADE_ROUTER_CONTRACT},
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
    sent_asset: Asset,
    swap_operations: Vec<SwapOperation>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "secret123".to_string(),
                denom_out: "secret456".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "secret123".to_string(),
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&snip20::ExecuteMsg::Send {
                        recipient: "shade_router".to_string(),
                        recipient_code_hash: Some("code_hash".to_string()),
                        amount: 100u128.into(),
                        msg: Some(to_binary(&shade_router::InvokeMsg::SwapTokensForExact {
                            path: vec![shade_router::Hop {
                                addr: "pool_1".to_string(),
                                code_hash: "code_hash".to_string(),
                            }],
                            expected_return: None,
                            recipient: None,
                        }).unwrap()),
                        memo: None,
                        padding: None,
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
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "secret456".to_string(),
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
/*
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "secret123".to_string(),
                denom_out: "secret456".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "pool_2".to_string(),
                denom_in: "secret456".to_string(),
                denom_out: "secret789".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::AstroportPoolSwap {
                        operation: SwapOperation {
                            pool: "pool_1".to_string(),
                            denom_in: "secret123".to_string(),
                            denom_out: "secret456".to_string(),
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
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::AstroportPoolSwap {
                        operation: SwapOperation {
                            pool: "pool_2".to_string(),
                            denom_in: "secret456".to_string(),
                            denom_out: "secret789".to_string(),
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
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "secret789".to_string(),
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
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::SwapOperationsEmpty),
    };
    "No Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(cw_utils::PaymentError::NoFunds{})),
    };
    "No Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
*/
fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");
    env.contract.code_hash = "code_hash".to_string();

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT.save(
        deps.as_mut().storage,
        &ContractInfo {
            address: Addr::unchecked("entry_point"),
            code_hash: "code_hash".to_string(),
        },
    )?;
    SHADE_ROUTER_CONTRACT.save(
        deps.as_mut().storage,
        &ContractInfo {
            address: Addr::unchecked("shade_router".to_string()),
            code_hash: "code_hash".to_string(),
        },
    )?;
    SHADE_POOL_CODE_HASH
        .save(deps.as_mut().storage, &"code_hash".to_string())
        .unwrap();

    REGISTERED_TOKENS
        .save(
            deps.as_mut().storage,
            Addr::unchecked("secret123".to_string()),
            &ContractInfo {
                address: Addr::unchecked("secret123".to_string()),
                code_hash: "code_hash".to_string(),
            },
        )
        .unwrap();

    REGISTERED_TOKENS
        .save(
            deps.as_mut().storage,
            Addr::unchecked("secret456".to_string()),
            &ContractInfo {
                address: Addr::unchecked("secret456".to_string()),
                code_hash: "code_hash".to_string(),
            },
        )
        .unwrap();

    REGISTERED_TOKENS
        .save(
            deps.as_mut().storage,
            Addr::unchecked("secret789".to_string()),
            &ContractInfo {
                address: Addr::unchecked("secret789".to_string()),
                code_hash: "code_hash".to_string(),
            },
        )
        .unwrap();

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_shade_protocol::contract::execute(
        deps.as_mut(),
        env,
        mock_info(&params.sent_asset.denom(), &vec![]),
        ExecuteMsg::Receive(Snip20ReceiveMsg {
            sender: Addr::unchecked(params.caller.clone()),
            from: Addr::unchecked(params.caller),
            amount: params.sent_asset.amount(),
            memo: None,
            msg: Some(to_binary(&Snip20HookMsg::Swap {
                operations: params.swap_operations.clone(),
            })?),
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
