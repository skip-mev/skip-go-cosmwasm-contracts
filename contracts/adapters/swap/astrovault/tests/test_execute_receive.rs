use astrovault::assets::pools::PoolInfoInput;
use astrovault::router::handle_msg::RouterReceiveMsg;
use astrovault::router::state::HopV2;
use astrovault::{assets::asset::AssetInfo, router::query_msg::RoutePoolType};
use cosmwasm_std::Uint128;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_json_binary, Addr, Coin, QuerierResult,
    ReplyOn::Never,
    SubMsg, SystemResult, WasmMsg, WasmQuery,
};
use skip::swap::{Cw20HookMsg, ExecuteMsg, SwapOperation};
use skip_go_swap_adapter_astrovault::{
    error::{ContractError, ContractResult},
    state::{ASTROVAULT_ROUTER_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - One swap operation (starting with cw20 token)

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
 */

// Define test parameters
struct Params {
    sender: String,
    caller: String,
    info_funds: Vec<Coin>,
    swap_operations: Vec<SwapOperation>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

pub const CW20_ADDR: &str = "neutron10dxyft3nv4vpxh5vrpn0xw8geej8dw3g39g7nqp8mrm307ypssksau29af";

// Test execute_swap
#[test_case(
    Params {
        sender: "entry_point".to_string(),
        caller: CW20_ADDR.to_string(),
        info_funds: vec![],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_3".to_string(),
                denom_in: CW20_ADDR.to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: CW20_ADDR.to_string(),
                    msg: to_json_binary(&
                        cw20::Cw20ExecuteMsg::Send { contract: "astrovault_router".to_string(), amount: Uint128::from(100u128), msg: to_json_binary(&RouterReceiveMsg::RouteV2 {
                            hops: vec![
                                HopV2::RatioHopInfo { pool:
                                    PoolInfoInput::Addr("pool_3".to_string()), from_asset_index: 0 }
                            ],
                            minimum_receive: None,
                            to: None,
                        })? }
                        )?,
                    funds: vec![],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_json_binary(&ExecuteMsg::TransferFundsBack {
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
        sender: "random".to_string(),
        caller: CW20_ADDR.to_string(),
        info_funds: vec![],
        swap_operations: vec![
            SwapOperation {
                pool: "pool_3".to_string(),
                denom_in: CW20_ADDR.to_string(),
                denom_out: "ua".to_string(),
                interface: None,
            }
        ],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]

fn test_execute_receive(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();
    let swap_ops = params.swap_operations.clone();

    // Create mock wasm handler to handle the swap adapter contract query
    let wasm_handler = move |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { contract_addr, .. } => {
                // the function queries the balance of the contract address
                if contract_addr == &CW20_ADDR.to_string() {
                    return SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                        to_json_binary(&cw20::BalanceResponse {
                            balance: Uint128::from(100u128),
                        })
                        .unwrap(),
                    ));
                }
                if contract_addr == "astrovault_router" {
                    let mut mock_route_pool_type_query_response = vec![];
                    if !swap_ops.is_empty() {
                        mock_route_pool_type_query_response.push(RoutePoolType {
                            pool_addr: "pool_3".to_string(),
                            pool_type: "hybrid".to_string(),
                            pool_asset_infos: vec![
                                AssetInfo::Token {
                                    contract_addr: CW20_ADDR.to_string(),
                                },
                                AssetInfo::NativeToken {
                                    denom: "ua".to_string(),
                                },
                            ],
                        });
                    }
                    SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                        to_json_binary(&mock_route_pool_type_query_response).unwrap(),
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

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, info_funds);

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;
    ASTROVAULT_ROUTER_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("astrovault_router"))?;

    // Call execute_swap with the given test parameters
    let res = skip_go_swap_adapter_astrovault::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: params.sender,
            amount: Uint128::from(100u128),
            msg: to_json_binary(&Cw20HookMsg::Swap {
                operations: params.swap_operations,
            })?,
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
