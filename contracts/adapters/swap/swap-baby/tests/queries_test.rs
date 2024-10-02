use skip_go_swap_adapter_swap_baby::contract::{instantiate, query};
use skip_go_swap_adapter_swap_baby::error::ContractError;
use skip_go_swap_adapter_swap_baby::msg::InstantiateMsg;
use skip_go_swap_adapter_swap_baby::swap_baby;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Coin, ContractResult, Decimal, QuerierResult, Uint128,
    WasmQuery,
};
use skip::swap::{Route, SwapOperation};
use std::str::FromStr;

struct TestCase {
    // name defines the test case name
    name: &'static str,
    // the skip query msg
    input: skip::swap::QueryMsg,
    // what the contract should forward to the router
    expected_router_input: swap_baby::QueryMsg,
    // what the router will reply back
    swap_baby_router_resp: Binary,
    // what the skip adapter contract should respond back
    expected_output: Result<Binary, ContractError>,
}

impl TestCase {
    fn new(
        name: &'static str,
        input: skip::swap::QueryMsg,
        expected_router_input: swap_baby::QueryMsg,
        router_response: impl Serialize,
        expected_output: Result<impl Serialize, ContractError>,
    ) -> Self {
        Self {
            name,
            input,
            expected_router_input,
            swap_baby_router_resp: to_json_binary(&router_response).unwrap(),
            expected_output: expected_output.map(|v| to_json_binary(&v).unwrap()),
        }
    }

    fn run_test(self) {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let msg = InstantiateMsg {
            entry_point_contract_address: "entry_point".to_string(),
            router_contract_address: "router".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

        let querier = move |msg: &WasmQuery| -> QuerierResult {
            match msg {
                WasmQuery::Smart {
                    contract_addr,
                    msg,
                } => {
                    assert_eq!(contract_addr, "router");
                    let msg = from_json::<swap_baby::QueryMsg>(msg)
                        .expect("unable to decode router message");
                    assert_eq!(
                        self.expected_router_input, msg,
                        "Test '{}': expected router request does not match the one from the adapter",
                        self.name,
                    );
                    QuerierResult::Ok(ContractResult::Ok(self.swap_baby_router_resp.clone()))
                }
                _ => panic!("unexpected query"),
            }
        };
        deps.querier.update_wasm(querier);
        let query_resp = query(deps.as_ref(), env, self.input);
        match self.expected_output {
            Ok(r) => {
                assert_eq!(r, query_resp.expect("no error wanted"));
            }
            Err(err) => {
                query_resp.expect_err("error wanted");
            }
        }
    }
}

#[test]
fn tests() {
    let tests = vec![
        TestCase::new(
            "simulate swap exact asset in",
            skip::swap::QueryMsg::SimulateSwapExactAssetIn {
                asset_in: Coin::new(100, "btc").into(),
                swap_operations: vec![
                    SwapOperation {
                        pool: "cw-btc-usdt".to_string(),
                        denom_in: "BTC".to_string(),
                        denom_out: "USDT".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "cw-eth-usdt".to_string(),
                        denom_in: "USDT".to_string(),
                        denom_out: "ETH".to_string(),
                        interface: None,
                    },
                ],
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
                input: Coin::new(100, "btc"),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-eth-usdt".to_string(),
                        denom: "ETH".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountInWithHopsResponse {
                coin_out: Uint128::new(10),
                fees: vec![],
                spot_price: Decimal::from_str("0.10").unwrap(),
            },
            Ok(skip::asset::Asset::Native(Coin::new(10, "ETH"))),
        ),
        // simulate swap exact asset in with metadata and price
        TestCase::new(
            "simulate swap exact asset in with metadata and price",
            skip::swap::QueryMsg::SimulateSwapExactAssetInWithMetadata {
                asset_in: Coin::new(1, "BTC").into(),
                swap_operations: vec![SwapOperation {
                    pool: "cw-btc-usdt".to_string(),
                    denom_in: "BTC".to_string(),
                    denom_out: "USDT".to_string(),
                    interface: None,
                }],
                include_spot_price: true,
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
                input: Coin::new(1, "BTC"),
                hops: vec![swap_baby::Hop {
                    pool: "cw-btc-usdt".to_string(),
                    denom: "USDT".to_string(),
                }],
            },
            swap_baby::QuerySimulateSwapExactAmountInWithHopsResponse {
                coin_out: Uint128::new(100_000),
                fees: vec![],
                spot_price: Decimal::from_str("100000").unwrap(),
            },
            Ok(skip::swap::SimulateSwapExactAssetInResponse {
                asset_out: Coin::new(100_000, "USDT").into(),
                spot_price: Some(Decimal::from_str("100000").unwrap()),
            }),
        ),
        TestCase::new(
            "simulate swap exact amount in with metadata and without price",
            skip::swap::QueryMsg::SimulateSwapExactAssetInWithMetadata {
                asset_in: Coin::new(1, "BTC").into(),
                swap_operations: vec![SwapOperation {
                    pool: "cw-btc-usdt".to_string(),
                    denom_in: "BTC".to_string(),
                    denom_out: "USDT".to_string(),
                    interface: None,
                }],
                include_spot_price: false,
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
                input: Coin::new(1, "BTC"),
                hops: vec![swap_baby::Hop {
                    pool: "cw-btc-usdt".to_string(),
                    denom: "USDT".to_string(),
                }],
            },
            swap_baby::QuerySimulateSwapExactAmountInWithHopsResponse {
                coin_out: Uint128::new(100_000),
                fees: vec![],
                spot_price: Decimal::from_str("100000").unwrap(),
            },
            Ok(skip::swap::SimulateSwapExactAssetInResponse {
                asset_out: Coin::new(100_000, "USDT").into(),
                spot_price: None,
            }),
        ),
        TestCase::new(
            "simulate swap exact amount out without metadata",
            skip::swap::QueryMsg::SimulateSwapExactAssetOut {
                asset_out: Coin::new(10, "ETH").into(),
                swap_operations: vec![
                    SwapOperation {
                        pool: "cw-btc-usdt".to_string(),
                        denom_in: "BTC".to_string(),
                        denom_out: "USDT".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "cw-eth-usdt".to_string(),
                        denom_in: "USDT".to_string(),
                        denom_out: "ETH".to_string(),
                        interface: None,
                    },
                ],
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountOutWithHops {
                want_out: Coin::new(10, "ETH").into(),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "BTC".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountOutWithHopsResponse {
                need_input: Uint128::new(1),
                fees: vec![],
                spot_price: Decimal::from_str("0.1").unwrap(),
            },
            Ok(skip::asset::Asset::Native(Coin::new(1, "BTC"))),
        ),
        TestCase::new(
            "simulate swap out with metadata and price",
            skip::swap::QueryMsg::SimulateSwapExactAssetOutWithMetadata {
                asset_out: Coin::new(10, "ETH").into(),
                swap_operations: vec![
                    SwapOperation {
                        pool: "cw-btc-usdt".to_string(),
                        denom_in: "BTC".to_string(),
                        denom_out: "USDT".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "cw-eth-usdt".to_string(),
                        denom_in: "USDT".to_string(),
                        denom_out: "ETH".to_string(),
                        interface: None,
                    },
                ],
                include_spot_price: true,
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountOutWithHops {
                want_out: Coin::new(10, "ETH").into(),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "BTC".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountOutWithHopsResponse {
                need_input: Uint128::new(1),
                fees: vec![],
                spot_price: Decimal::from_str("0.1").unwrap(),
            },
            Ok(skip::swap::SimulateSwapExactAssetOutResponse {
                asset_in: Coin::new(1, "BTC").into(),
                spot_price: Some(Decimal::from_str("0.1").unwrap()),
            }),
        ),
        TestCase::new(
            "simulate swap out with metadata and without price",
            skip::swap::QueryMsg::SimulateSwapExactAssetOutWithMetadata {
                asset_out: Coin::new(10, "ETH").into(),
                swap_operations: vec![
                    SwapOperation {
                        pool: "cw-btc-usdt".to_string(),
                        denom_in: "BTC".to_string(),
                        denom_out: "USDT".to_string(),
                        interface: None,
                    },
                    SwapOperation {
                        pool: "cw-eth-usdt".to_string(),
                        denom_in: "USDT".to_string(),
                        denom_out: "ETH".to_string(),
                        interface: None,
                    },
                ],
                include_spot_price: false,
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountOutWithHops {
                want_out: Coin::new(10, "ETH").into(),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "BTC".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-btc-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountOutWithHopsResponse {
                need_input: Uint128::new(1),
                fees: vec![],
                spot_price: Decimal::from_str("0.1").unwrap(),
            },
            Ok(skip::swap::SimulateSwapExactAssetOutResponse {
                asset_in: Coin::new(1, "BTC").into(),
                spot_price: None,
            }),
        ),
        TestCase::new(
            "simulate smart swap exact asset in",
            skip::swap::QueryMsg::SimulateSmartSwapExactAssetIn {
                asset_in: Coin::new(1, "BTC").into(),
                routes: vec![Route {
                    offer_asset: Coin::new(1, "BTC").into(),
                    operations: vec![
                        SwapOperation {
                            pool: "cw-eth-btc".to_string(),
                            denom_in: "BTC".to_string(),
                            denom_out: "ETH".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "cw-eth-usdt".to_string(),
                            denom_in: "ETH".to_string(),
                            denom_out: "USDT".to_string(),
                            interface: None,
                        },
                    ],
                }],
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
                input: Coin::new(1, "BTC"),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-eth-btc".to_string(),
                        denom: "ETH".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-eth-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountInWithHopsResponse {
                coin_out: Uint128::new(100_000),
                fees: vec![],
                spot_price: Decimal::from_str("100000").unwrap(),
            },
            Ok(skip::asset::Asset::Native(Coin::new(100_000, "USDT"))),
        ),
        TestCase::new(
            "simulate smart swap exact asset in with metadata and price",
            skip::swap::QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
                asset_in: Coin::new(1, "BTC").into(),
                routes: vec![Route {
                    offer_asset: Coin::new(1, "BTC").into(),
                    operations: vec![
                        SwapOperation {
                            pool: "cw-eth-btc".to_string(),
                            denom_in: "BTC".to_string(),
                            denom_out: "ETH".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "cw-eth-usdt".to_string(),
                            denom_in: "ETH".to_string(),
                            denom_out: "USDT".to_string(),
                            interface: None,
                        },
                    ],
                }],
                include_spot_price: true,
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
                input: Coin::new(1, "BTC"),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-eth-btc".to_string(),
                        denom: "ETH".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-eth-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountInWithHopsResponse {
                coin_out: Uint128::new(100_000),
                fees: vec![],
                spot_price: Decimal::from_str("100000").unwrap(),
            },
            Ok(skip::swap::SimulateSmartSwapExactAssetInResponse {
                asset_out: Coin::new(100_000, "USDT").into(),
                spot_price: Some(Decimal::from_str("100000").unwrap()),
            }),
        ),
        TestCase::new(
            "simulate smart swap exact asset in with metadata without price",
            skip::swap::QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
                asset_in: Coin::new(1, "BTC").into(),
                routes: vec![Route {
                    offer_asset: Coin::new(1, "BTC").into(),
                    operations: vec![
                        SwapOperation {
                            pool: "cw-eth-btc".to_string(),
                            denom_in: "BTC".to_string(),
                            denom_out: "ETH".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "cw-eth-usdt".to_string(),
                            denom_in: "ETH".to_string(),
                            denom_out: "USDT".to_string(),
                            interface: None,
                        },
                    ],
                }],
                include_spot_price: false,
            },
            swap_baby::QueryMsg::SimulateSwapExactAmountInWithHops {
                input: Coin::new(1, "BTC"),
                hops: vec![
                    swap_baby::Hop {
                        pool: "cw-eth-btc".to_string(),
                        denom: "ETH".to_string(),
                    },
                    swap_baby::Hop {
                        pool: "cw-eth-usdt".to_string(),
                        denom: "USDT".to_string(),
                    },
                ],
            },
            swap_baby::QuerySimulateSwapExactAmountInWithHopsResponse {
                coin_out: Uint128::new(100_000),
                fees: vec![],
                spot_price: Decimal::from_str("100000").unwrap(),
            },
            Ok(skip::swap::SimulateSmartSwapExactAssetInResponse {
                asset_out: Coin::new(100_000, "USDT").into(),
                spot_price: None,
            }),
        ),
    ];

    for t in tests {
        t.run_test();
    }
}
