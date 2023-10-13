use cosmwasm_std::testing::{mock_dependencies_with_balances, mock_env, mock_info};
use cosmwasm_std::{
    to_binary, Addr, Coin, ContractResult, CosmosMsg, QuerierResult, ReplyOn, SubMsg, SystemResult,
    Timestamp, WasmMsg, WasmQuery,
};
use skip::entry_point::{Action, Affiliate, ExecuteMsg};
use skip::swap::{Swap, SwapExactCoinIn, SwapOperation};
use skip_api_entry_point::error::ContractError;
use skip_api_entry_point::state::{IBC_TRANSFER_CONTRACT_ADDRESS, SWAP_VENUE_MAP};

pub struct Params {
    info_funds: Vec<Coin>,
    user_swap: Swap,
    min_coin: Coin,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Helper function for all SWAP_AND_ACTION msgs
pub fn test_execute_swap_and_action(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock wasm handler to handle the swap adapter contract query
    // Will always return 200_000 osmo
    let wasm_handler = |query: &WasmQuery| -> QuerierResult {
        match query {
            WasmQuery::Smart { .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&Coin::new(200_000, "osmo")).unwrap(),
            )),
            _ => panic!("Unsupported query: {:?}", query),
        }
    };

    // Update querier with mock wasm handler
    deps.querier.update_wasm(wasm_handler);

    // Create mock env with parameters that make testing easier
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("entry_point");
    env.block.time = Timestamp::from_nanos(100);

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info("swapper", info_funds);

    // Store the swap venue adapter contract address
    let swap_venue_adapter = Addr::unchecked("swap_venue_adapter");
    SWAP_VENUE_MAP
        .save(
            deps.as_mut().storage,
            "swap_venue_name",
            &swap_venue_adapter,
        )
        .unwrap();

    // Store the ibc transfer adapter contract address
    let ibc_transfer_adapter = Addr::unchecked("ibc_transfer_adapter");
    IBC_TRANSFER_CONTRACT_ADDRESS
        .save(deps.as_mut().storage, &ibc_transfer_adapter)
        .unwrap();

    let recovery_addr = Addr::unchecked("recovery_address");
    // Call execute_swap_and_action with the given test case params
    let res = skip_api_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info.clone(),
        ExecuteMsg::AxelarSwapAndAction {
            user_swap: params.user_swap,
            min_coin: params.min_coin,
            timeout_timestamp: params.timeout_timestamp,
            post_swap_action: params.post_swap_action,
            affiliates: params.affiliates,
            recovery_addr,
        },
    );

    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the number of messages in the response is correct
            assert_eq!(
                res.messages.len(),
                params.expected_messages.len(),
                "expected {:?} messages, but got {:?}",
                params.expected_messages.len(),
                res.messages.len()
            );

            // Assert the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages,);
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
}

// Test new ExecuteMsg::AxelarSwapAndAction msg and verify it correctly sends the ExecuteMsg::SwapAndAction
#[test]
pub fn successful_swap_and_action() {
    let params = Params {
        info_funds: vec![Coin::new(1_000_000, "untrn")],
        user_swap: Swap::SwapExactCoinIn(SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
            }],
        }),
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 101,
        post_swap_action: Action::BankSend {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_binary(&ExecuteMsg::SwapAndAction {
                    user_swap: Swap::SwapExactCoinIn(SwapExactCoinIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                        }],
                    }),
                    min_coin: Coin::new(1_000_000, "osmo"),
                    timeout_timestamp: 101,
                    post_swap_action: Action::BankSend {
                        to_address: "to_address".to_string(),
                    },
                    affiliates: vec![],
                })
                .unwrap(),
                funds: vec![Coin::new(1000000, "untrn")],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }],
        expected_error: None,
    };

    test_execute_swap_and_action(params);
}

// Error should be handled with reply message so it should not return an Timeout contract error as it would if you called ExecuteMsg::SwapAndAction
#[test]
pub fn timeout_error_passes() {
    let params = Params {
        info_funds: vec![Coin::new(1_000_000, "untrn")],
        user_swap: Swap::SwapExactCoinIn(SwapExactCoinIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
            }],
        }),
        min_coin: Coin::new(1_000_000, "osmo"),
        timeout_timestamp: 0,
        post_swap_action: Action::BankSend {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_binary(&ExecuteMsg::SwapAndAction {
                    user_swap: Swap::SwapExactCoinIn(SwapExactCoinIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                        }],
                    }),
                    min_coin: Coin::new(1_000_000, "osmo"),
                    timeout_timestamp: 0,
                    post_swap_action: Action::BankSend {
                        to_address: "to_address".to_string(),
                    },
                    affiliates: vec![],
                })
                .unwrap(),
                funds: vec![Coin::new(1000000, "untrn")],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }],
        expected_error: None,
    };

    test_execute_swap_and_action(params);
}
