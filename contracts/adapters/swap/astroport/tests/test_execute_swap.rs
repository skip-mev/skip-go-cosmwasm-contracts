use astroport::{
    asset::AssetInfo,
    router::{ExecuteMsg as RouterExecuteMsg, SwapOperation as AstroportSwapOperation},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Coin,
    ReplyOn::Never,
    SubMsg, WasmMsg,
};
use skip::{
    asset::Asset,
    error::SkipError::Payment,
    swap::{ExecuteMsg, SwapOperation},
};
use skip_api_swap_adapter_astroport::{
    error::{ContractError, ContractResult},
    state::{ENTRY_POINT_CONTRACT_ADDRESS, ROUTER_CONTRACT_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Swap Operation
    - Multiple Swap Operations
    - No Swap Operations (This is prevented in the entry point contract; and will fail on Astroport router if attempted)

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - No Coin Sent
    - More Than One Coin Sent

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
        info_funds: vec![Coin::new(100, "os")],
        sent_asset: Asset::Native(Coin::new(100, "os")),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "router_contract".to_string(),
                    msg: to_binary(&RouterExecuteMsg::ExecuteSwapOperations {
                        operations: vec![
                            AstroportSwapOperation::AstroSwap {
                                offer_asset_info: AssetInfo::NativeToken {
                                    denom: "os".to_string(),
                                },
                                ask_asset_info: AssetInfo::NativeToken {
                                    denom: "ua".to_string(),
                                },
                            }
                        ],
                        minimum_receive: None,
                        to: None,
                        max_spread: None,
                    })?,
                    funds: vec![Coin::new(100, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
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
    "One Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        sent_asset: Asset::Native(Coin::new(100, "os")),
        swap_operations: vec![
            SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
            },
            SwapOperation {
                pool: "pool_2".to_string(),
                denom_in: "ua".to_string(),
                denom_out: "un".to_string(),
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "router_contract".to_string(),
                    msg: to_binary(&RouterExecuteMsg::ExecuteSwapOperations {
                        operations: vec![
                            AstroportSwapOperation::AstroSwap {
                                offer_asset_info: AssetInfo::NativeToken {
                                    denom: "os".to_string(),
                                },
                                ask_asset_info: AssetInfo::NativeToken {
                                    denom: "ua".to_string(),
                                },
                            },
                            AstroportSwapOperation::AstroSwap {
                                offer_asset_info: AssetInfo::NativeToken {
                                    denom: "ua".to_string(),
                                },
                                ask_asset_info: AssetInfo::NativeToken {
                                    denom: "un".to_string(),
                                },
                            }
                        ],
                        minimum_receive: None,
                        to: None,
                        max_spread: None,
                    })?,
                    funds: vec![Coin::new(100, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
                        return_denom: "un".to_string(),
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
        info_funds: vec![Coin::new(100, "os")],
        sent_asset: Asset::Native(Coin::new(100, "os")),
        swap_operations: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "router_contract".to_string(),
                    msg: to_binary(&RouterExecuteMsg::ExecuteSwapOperations {
                        operations: vec![],
                        minimum_receive: None,
                        to: None,
                        max_spread: None,
                    })?,
                    funds: vec![Coin::new(100, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
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
        expected_error: Some(ContractError::SwapOperationsEmpty),
    };
    "No Swap Operations")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        sent_asset: Asset::Native(Coin::new(100, "os")),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Payment(cw_utils::PaymentError::NoFunds{}))),
    };
    "No Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "un"),
            Coin::new(100, "os"),
        ],
        sent_asset: Asset::Native(Coin::new(100, "os")),
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Skip(Payment(cw_utils::PaymentError::MultipleDenoms{}))),
    };
    "More Than One Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![
            Coin::new(100, "un"),
            Coin::new(100, "os"),
        ],
        sent_asset: Asset::Native(Coin::new(100, "os")),
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

    // Store the router contract address
    ROUTER_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("router_contract"))?;

    // Call execute_swap with the given test parameters
    let res = skip_api_swap_adapter_astroport::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Swap {
            sent_asset: params.sent_asset,
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
