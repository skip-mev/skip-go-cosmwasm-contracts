use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin,
    ReplyOn::Never,
    SubMsg,  WasmMsg,
};
use oraiswap_v3::{percentage::Percentage, FeeTier, PoolKey};
use skip::swap::{ExecuteMsg, SwapOperation};
use skip_api_swap_adapter_oraidex::{
    error::ContractResult,
    state::{ENTRY_POINT_CONTRACT_ADDRESS, ORAIDEX_ROUTER_ADDRESS},
};

use oraiswap::{
    asset::AssetInfo,
    mixed_router::{ExecuteMsg as OraidexRouterExecuteMsg, SwapOperation as OraidexSwapOperation},
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One Swap Operation
    - Multiple Swap Operations
    - No Swap Operations (This is prevented in the entry point contract; and will fail on Osmosis module if attempted)

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
        info_funds: vec![Coin::new(100, "orai")],
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "orai".to_string(),
                denom_out: "atom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "oraidex_router".to_string(),
                    msg: to_json_binary(&OraidexRouterExecuteMsg::ExecuteSwapOperations { operations: vec![OraidexSwapOperation::OraiSwap { offer_asset_info: AssetInfo::Token { contract_addr: Addr::unchecked("orai") }, ask_asset_info:AssetInfo::Token { contract_addr: Addr::unchecked("atom") } } ], minimum_receive: None, to: Some(Addr::unchecked("entry_point")) })?,
                    funds: vec![
                        Coin::new(100, "orai")
                    ],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            }
        ],
        expected_error_string: "".to_string(),
    };
"One Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "orai")],
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "orai".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "uatom-usdt-3000000000-10".to_string(),
                denom_in: "uatom".to_string(),
                denom_out: "usdt".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "oraidex_router".to_string(),
                    msg: to_json_binary(&OraidexRouterExecuteMsg::ExecuteSwapOperations { 
                        operations:  vec![
                            OraidexSwapOperation::OraiSwap { offer_asset_info: AssetInfo::Token { contract_addr: Addr::unchecked("orai") }, ask_asset_info:AssetInfo::Token { contract_addr: Addr::unchecked("uatom") } },
                            OraidexSwapOperation::SwapV3 { pool_key: PoolKey{token_x: "uatom".to_string(), token_y: "usdt".to_string(), fee_tier: FeeTier {fee: Percentage(3000000000), tick_spacing: 10}}, x_to_y: true }],
                        minimum_receive: None, 
                        to: Some(Addr::unchecked("entry_point")) })?,
                    funds: vec![
                        Coin::new(100, "orai")
                    ],
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
        info_funds: vec![Coin::new(100, "orai")],
        swap_operations: vec![],
        expected_messages: vec![
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
                pool: "1".to_string(),
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
                pool: "1".to_string(),
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
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![
            SwapOperation {
                pool: "tokenx-tokeny-100".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "Generic error: Invalid v3 pool_id, require exactly 4 fields".to_string(),
    };
    "Invalid Pool ID Conversion For Swap Operations - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "os")],
        swap_operations: vec![
            SwapOperation {
                pool: "tokenx-tokeny-abc-def".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error_string: "Generic error: Invalid fee in v3 pool".to_string(),
    };
    "Invalid Pool ID Conversion For Swap Operations, cannot parse string to uint - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![
            Coin::new(100, "uxprt"),
            // Coin::new(100, "os"),
        ],
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uxprt".to_string(),
                denom_out: "stk/uxprt".to_string(),
                interface: None,
            }
        ],
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
    ORAIDEX_ROUTER_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("oraidex_router"))?;

    // Call execute_swap with the given test parameters
    let res = skip_api_swap_adapter_oraidex::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Swap {
            operations: params.swap_operations.clone(),
        },
    );
    println!("{:?}", res);

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
