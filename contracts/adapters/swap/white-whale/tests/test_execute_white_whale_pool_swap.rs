use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_json_binary, Addr, Coin, Decimal, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg};
use skip::swap::{ExecuteMsg, SwapOperation};
use skip_api_swap_adapter_white_whale::error::{ContractError, ContractResult};
use test_case::test_case;
use white_whale_std::pool_network::{
    asset::{Asset as WhiteWhaleAsset, AssetInfo},
    pair::{Cw20HookMsg as WhiteWhalePairCw20HookMsg, ExecuteMsg as WhiteWhalePairExecuteMsg},
};

/*
Test Cases:

Expect Success
    - Native Swap Operation
    - Cw20 Swap Operation

Expect Error
    - No Native Offer Asset In Contract Balance To Swap
    - No Cw20 Offer Asset In Contract Balance To Swap
    - Unauthorized Caller

 */

// Define test parameters
struct Params {
    caller: String,
    contract_balance: Vec<Coin>,
    swap_operation: SwapOperation,
    expected_message: Option<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![Coin::new(100, "os")],
        swap_operation: SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            },
        expected_message: Some(SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "pool_1".to_string(),
                    msg: to_json_binary(&WhiteWhalePairExecuteMsg::Swap {
                        offer_asset: WhiteWhaleAsset {
                            info: AssetInfo::NativeToken {
                                denom: "os".to_string(),
                            },
                            amount: Uint128::new(100),
                        },
                        belief_price: None,
                        max_spread: Some(Decimal::percent(50)),
                        to: None,
                    })?,
                    funds: vec![Coin::new(100, "os")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            }),
        expected_error: None,
    };
    "Native Swap Operation")]
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![],
        swap_operation: SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "neutron123".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            },
        expected_message: Some(SubMsg {
            id: 0,
            msg: WasmMsg::Execute {
                contract_addr: "neutron123".to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: "pool_1".to_string(),
                    amount: Uint128::from(100u128),
                    msg: to_json_binary(&WhiteWhalePairCw20HookMsg::Swap {
                        belief_price: None,
                        max_spread: Some(Decimal::percent(50)),
                        to: None,
                    })?,
                })?,
                funds: vec![],
            }.into(),
            gas_limit: None,
            reply_on: Never,
        }),
        expected_error: None,
    };
    "Cw20 Swap Operation")]
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![],
        swap_operation: SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            },
        expected_message: None,
        expected_error: Some(ContractError::NoOfferAssetAmount),
    };
    "No Native Offer Asset In Contract Balance To Swap")]
#[test_case(
    Params {
        caller: "swap_contract_address".to_string(),
        contract_balance: vec![],
        swap_operation: SwapOperation {
                pool: "pool_1".to_string(),
                denom_in: "randomcw20".to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            },
        expected_message: None,
        expected_error: Some(ContractError::NoOfferAssetAmount),
    };
    "No Cw20 Offer Asset In Contract Balance To Swap")]
#[test_case(
    Params {
        caller: "random".to_string(),
        contract_balance: vec![
            Coin::new(100, "un"),
        ],
        swap_operation: SwapOperation{
            pool: "".to_string(),
            denom_in: "".to_string(),
            denom_out: "".to_string(),
            interface: None,
        },
        expected_message: None,
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
fn test_execute_white_whale_pool_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps =
        mock_dependencies_with_balances(&[("swap_contract_address", &params.contract_balance)]);

    // Create mock wasm handler to handle the cw20 balance queries
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                if contract_addr == "neutron123" {
                    SystemResult::Ok(
                        ContractResult::Ok(
                            to_json_binary(&BalanceResponse {
                                balance: Uint128::from(100u128),
                            })
                            .unwrap(),
                        )
                        .into(),
                    )
                } else {
                    SystemResult::Ok(
                        ContractResult::Ok(
                            to_json_binary(&BalanceResponse {
                                balance: Uint128::from(0u128),
                            })
                            .unwrap(),
                        )
                        .into(),
                    )
                }
            }
            _ => panic!("Unsupported query: {:?}", query),
        }
    };
    deps.querier.update_wasm(wasm_handler);

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Create mock info
    let info = mock_info(&params.caller, &[]);

    // Call execute_white_whale_pool_swap with the given test parameters
    let res = skip_api_swap_adapter_white_whale::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::WhiteWhalePoolSwap {
            operation: params.swap_operation,
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
            assert_eq!(res.messages[0], params.expected_message.unwrap());
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
