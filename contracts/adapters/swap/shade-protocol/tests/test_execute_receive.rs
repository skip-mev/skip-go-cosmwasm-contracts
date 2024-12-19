use core::panic;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Coin, ContractInfo, ContractResult as SystemContractResult, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20Coin};
use secret_skip::{asset::Asset, snip20, swap::SwapOperation};
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
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "secret123".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![

            SubMsg {
                id: 0,
                gas_limit: None,
                reply_on: Never,
                msg: WasmMsg::Execute {
                    contract_addr: "secret123".to_string(),
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&snip20::ExecuteMsg::Send {
                        amount: Uint128::from(100u128),
                        recipient: "shade_router".to_string(),
                        recipient_code_hash: Some("code_hash".to_string()),
                        memo: None,
                        padding: None,
                        msg: Some(to_binary(&shade_router::InvokeMsg::SwapTokensForExact {
                            path: vec![shade_router::Hop {
                                addr: "pool_1".to_string(),
                                code_hash: "code_hash".to_string(),
                            }],
                            expected_return: None,
                            recipient: None,
                        })?),
                    })?,
                    funds: vec![],
                }.into(),
            },
            SubMsg {
                id: 0,
                gas_limit: None,
                reply_on: Never,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    code_hash: "code_hash".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
                        swapper: Addr::unchecked("entry_point"),
                        return_denom: "ua".to_string(),
                    })?,
                    funds: vec![],
                }.into(),
            },
        ],
        expected_error: None,
    };
    "One Swap Operation - Snip20 In & Out")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "un"),
        ],
        sent_asset: Asset::Cw20(Cw20Coin {
            address: "secret123".to_string(),
            amount: 100u128.into()
        }),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::SwapOperationsEmpty),
    };
    "Coin sent with cw20 - Expect Error")]
fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "secret123" {
                    SystemResult::Ok(SystemContractResult::Ok(
                        to_binary(&BalanceResponse {
                            balance: 100u128.into(),
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
    env.contract.code_hash = "code_hash".to_string();

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info(params.sent_asset.denom(), info_funds);

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
            address: Addr::unchecked("shade_router"),
            code_hash: "code_hash".to_string(),
        },
    )?;
    SHADE_POOL_CODE_HASH.save(deps.as_mut().storage, &"code_hash".to_string())?;
    REGISTERED_TOKENS.save(
        deps.as_mut().storage,
        Addr::unchecked("secret123"),
        &ContractInfo {
            address: Addr::unchecked("secret123"),
            code_hash: "code_hash".to_string(),
        },
    )?;

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_shade_protocol::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Receive(snip20::Snip20ReceiveMsg {
            sender: Addr::unchecked(params.caller),
            amount: params.sent_asset.amount(),
            from: Addr::unchecked("entry_point".to_string()),
            memo: None,
            msg: Some(to_binary(&Snip20HookMsg::Swap {
                operations: params.swap_operations,
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
