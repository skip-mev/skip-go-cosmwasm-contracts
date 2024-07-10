use std::marker::PhantomData;
use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, from_json, OwnedDeps, StdResult};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use pryzm_std::types::cosmos::base::v1beta1::Coin as CosmosCoin;
use pryzm_std::types::pryzm::amm::v1::{
    QuerySimulateBatchSwapRequest, QuerySimulateBatchSwapResponse, QuerySpotPriceRequest,
    QuerySpotPriceResponse, SwapStep, SwapType,
};
use pryzm_std::types::pryzm::icstaking::v1::{
    QuerySimulateStakeRequest, QuerySimulateStakeResponse,
};

use skip::asset::Asset;
use skip::error::SkipError;
use skip::swap::{QueryMsg, Route, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation};
use skip_api_swap_adapter_pryzm::contract;
use skip_api_swap_adapter_pryzm::error::ContractError;

use crate::mock::MockQuerier;

mod mock;

#[test]
fn test_simulate_exact_asset_in() {
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: setup_mocks(),
        custom_query_type: PhantomData,
    };

    // empty swap operations
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in: Asset::Native(coin(1000, "ibc/uatom")),
            swap_operations: vec![],
        },
    );
    assert!(res.is_err());
    assert!(matches!(
        res.err().unwrap(),
        ContractError::SwapOperationsEmpty
    ));

    // invalid asset provided
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in: Asset::Native(coin(1000, "ibc/uosmo")),
            swap_operations: vec![SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "ibc/uatom".to_string(),
                denom_out: "c:uatom".to_string(),
                interface: None,
            }],
        },
    );
    assert!(res.is_err());
    assert!(matches!(
        res.err().unwrap(),
        ContractError::CoinInDenomMismatch
    ));

    // valid stake operation
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in: Asset::Native(coin(1000, "ibc/uatom")),
            swap_operations: vec![SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "ibc/uatom".to_string(),
                denom_out: "c:uatom".to_string(),
                interface: None,
            }],
        },
    );
    assert!(res.is_ok());
    let output: StdResult<Asset> = from_json(res.unwrap());
    assert!(output.is_ok());
    let token_out = output.unwrap();
    assert_eq!("c:uatom", token_out.denom());
    assert_eq!(950, token_out.amount().u128());

    // valid multi step swap
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetIn {
            asset_in: Asset::Native(coin(2000, "ibc/uosmo")),
            swap_operations: vec![
                SwapOperation {
                    pool: "amm:1".to_string(),
                    denom_in: "ibc/uosmo".to_string(),
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
                    denom_out: "y:uatom:30Sep2024".to_string(),
                    interface: None,
                },
            ],
        },
    );
    assert!(res.is_ok());
    let output: StdResult<Asset> = from_json(res.unwrap());
    assert!(output.is_ok());
    let token_out = output.unwrap();
    assert_eq!("y:uatom:30Sep2024", token_out.denom());
    assert_eq!(1200, token_out.amount().u128());

    // get with spot price
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in: Asset::Native(coin(2000, "ibc/uosmo")),
            swap_operations: vec![
                SwapOperation {
                    pool: "amm:1".to_string(),
                    denom_in: "ibc/uosmo".to_string(),
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
                    denom_out: "y:uatom:30Sep2024".to_string(),
                    interface: None,
                },
            ],
            include_spot_price: true,
        },
    );
    assert!(res.is_ok());
    let output: StdResult<SimulateSwapExactAssetInResponse> = from_json(res.unwrap());
    assert!(output.is_ok());
    let response = output.unwrap();
    let token_out = response.asset_out;
    assert_eq!("y:uatom:30Sep2024", token_out.denom());
    assert_eq!(1200, token_out.amount().u128());
    assert_eq!(
        Decimal::from_str("0.600000000125").unwrap(),
        response.spot_price.unwrap()
    );
}

#[test]
fn test_simulate_exact_asset_out() {
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: setup_mocks(),
        custom_query_type: PhantomData,
    };

    // empty swap operations
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out: Asset::Native(coin(1200, "y:uatom:30Sep2024")),
            swap_operations: vec![],
        },
    );
    assert!(res.is_err());
    assert!(matches!(
        res.err().unwrap(),
        ContractError::SwapOperationsEmpty
    ));

    // invalid asset provided
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out: Asset::Native(coin(1000, "c:uosmo")),
            swap_operations: vec![SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "ibc/uatom".to_string(),
                denom_out: "c:uatom".to_string(),
                interface: None,
            }],
        },
    );
    assert!(res.is_err());
    assert!(matches!(
        res.err().unwrap(),
        ContractError::CoinOutDenomMismatch
    ));

    // valid stake operation
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out: Asset::Native(coin(950, "c:uatom")),
            swap_operations: vec![SwapOperation {
                pool: "icstaking:uatom:channel-0".to_string(),
                denom_in: "ibc/uatom".to_string(),
                denom_out: "c:uatom".to_string(),
                interface: None,
            }],
        },
    );
    assert!(res.is_ok());
    let output: StdResult<Asset> = from_json(res.unwrap());
    assert!(output.is_ok());
    let token_in = output.unwrap();
    assert_eq!("ibc/uatom", token_in.denom());
    assert_eq!(1000, token_in.amount().u128());

    // valid multi step swap
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetOut {
            asset_out: Asset::Native(coin(1200, "y:uatom:30Sep2024")),
            swap_operations: vec![
                SwapOperation {
                    pool: "amm:1".to_string(),
                    denom_in: "ibc/uosmo".to_string(),
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
                    denom_out: "y:uatom:30Sep2024".to_string(),
                    interface: None,
                },
            ],
        },
    );
    assert!(res.is_ok());
    let output: StdResult<Asset> = from_json(res.unwrap());
    assert!(output.is_ok());
    let token_in = output.unwrap();
    assert_eq!("ibc/uosmo", token_in.denom());
    assert_eq!(2000, token_in.amount().u128());

    // get with spot price
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out: Asset::Native(coin(1200, "y:uatom:30Sep2024")),
            swap_operations: vec![
                SwapOperation {
                    pool: "amm:1".to_string(),
                    denom_in: "ibc/uosmo".to_string(),
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
                    denom_out: "y:uatom:30Sep2024".to_string(),
                    interface: None,
                },
            ],
            include_spot_price: true,
        },
    );
    assert!(res.is_ok());
    let output: StdResult<SimulateSwapExactAssetOutResponse> = from_json(res.unwrap());
    assert!(output.is_ok());
    let response = output.unwrap();
    let token_in = response.asset_in;
    assert_eq!("ibc/uosmo", token_in.denom());
    assert_eq!(2000, token_in.amount().u128());
    assert_eq!(
        Decimal::from_str("0.600000000125").unwrap(),
        response.spot_price.unwrap()
    );
}

#[test]
fn test_simulate_smart_swap() {
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: setup_mocks(),
        custom_query_type: PhantomData,
    };

    // empty routes
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSmartSwapExactAssetIn {
            asset_in: Asset::Native(coin(1000, "ibc/uatom")),
            routes: vec![],
        },
    );
    assert!(res.is_err());
    assert!(matches!(
        res.err().unwrap(),
        ContractError::Skip(SkipError::RoutesEmpty)
    ));

    // valid multi step swap
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSmartSwapExactAssetIn {
            asset_in: Asset::Native(coin(5000, "ibc/uosmo")),
            routes: vec![
                Route {
                    offer_asset: Asset::Native(coin(2000, "ibc/uosmo")),
                    operations: vec![
                        SwapOperation {
                            pool: "amm:1".to_string(),
                            denom_in: "ibc/uosmo".to_string(),
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
                            denom_out: "y:uatom:30Sep2024".to_string(),
                            interface: None,
                        },
                    ],
                },
                Route {
                    offer_asset: Asset::Native(coin(3000, "ibc/uosmo")),
                    operations: vec![
                        SwapOperation {
                            pool: "amm:1".to_string(),
                            denom_in: "ibc/uosmo".to_string(),
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
                            pool: "amm:4".to_string(),
                            denom_in: "ibc/uatom".to_string(),
                            denom_out: "c:uatom".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "amm:3".to_string(),
                            denom_in: "c:uatom".to_string(),
                            denom_out: "y:uatom:30Sep2024".to_string(),
                            interface: None,
                        },
                    ],
                }
            ],
        },
    );
    assert!(res.is_ok());
    let output: StdResult<Asset> = from_json(res.unwrap());
    assert!(output.is_ok());
    let token_out = output.unwrap();
    assert_eq!("y:uatom:30Sep2024", token_out.denom());
    assert_eq!(2905, token_out.amount().u128());

    // get with spot price
    let res = contract::query(
        deps.as_mut().as_ref(),
        mock_env(),
        QueryMsg::SimulateSmartSwapExactAssetInWithMetadata {
            asset_in: Asset::Native(coin(5000, "ibc/uosmo")),
            routes: vec![
                Route {
                    offer_asset: Asset::Native(coin(2000, "ibc/uosmo")),
                    operations: vec![
                        SwapOperation {
                            pool: "amm:1".to_string(),
                            denom_in: "ibc/uosmo".to_string(),
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
                            denom_out: "y:uatom:30Sep2024".to_string(),
                            interface: None,
                        },
                    ],
                },
                Route {
                    offer_asset: Asset::Native(coin(3000, "ibc/uosmo")),
                    operations: vec![
                        SwapOperation {
                            pool: "amm:1".to_string(),
                            denom_in: "ibc/uosmo".to_string(),
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
                            pool: "amm:4".to_string(),
                            denom_in: "ibc/uatom".to_string(),
                            denom_out: "c:uatom".to_string(),
                            interface: None,
                        },
                        SwapOperation {
                            pool: "amm:3".to_string(),
                            denom_in: "c:uatom".to_string(),
                            denom_out: "y:uatom:30Sep2024".to_string(),
                            interface: None,
                        },
                    ],
                }
            ],
            include_spot_price: true,
        },
    );
    assert!(res.is_ok());
    let output: StdResult<SimulateSwapExactAssetInResponse> = from_json(res.unwrap());
    assert!(output.is_ok());
    let response = output.unwrap();
    let token_out = response.asset_out;
    assert_eq!("y:uatom:30Sep2024", token_out.denom());
    assert_eq!(2905, token_out.amount().u128());

    // weighted spot price:
    // 2000 with 0.600000000125
    // 3000 with 0.56842105275
    assert_eq!(
        Decimal::from_str("0.5810526317").unwrap(),
        response.spot_price.unwrap()
    );
}

fn setup_mocks() -> MockQuerier {
    let mut querier = MockQuerier::new();

    mock_stake_given_in(&mut querier, "uatom", "channel-0", "1000", "950");
    mock_stake_given_out(&mut querier, "uatom", "channel-0", "950", "1000");
    mock_stake_given_in(
        &mut querier,
        "uatom",
        "channel-0",
        "1000000000000000000",
        "950000000000000000",
    );

    mock_batch_swap_given_in(
        &mut querier,
        vec![
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
        "1000",
        "ibc/uatom",
    );
    mock_batch_swap_given_out(
        &mut querier,
        vec![
            SwapStep {
                pool_id: 2,
                token_in: "ibc/uusdc".to_string(),
                token_out: "ibc/uatom".to_string(),
                amount: Some("1000".to_string()),
            },
            SwapStep {
                pool_id: 1,
                token_in: "ibc/uosmo".to_string(),
                token_out: "ibc/uusdc".to_string(),
                amount: None,
            },
        ],
        "2000",
        "ibc/uosmo",
    );

    mock_batch_swap_given_in(
        &mut querier,
        vec![SwapStep {
            pool_id: 3,
            token_in: "c:uatom".to_string(),
            token_out: "y:uatom:30Sep2024".to_string(),
            amount: Some("950".to_string()),
        }],
        "1200",
        "y:uatom:30Sep2024",
    );
    mock_batch_swap_given_out(
        &mut querier,
        vec![SwapStep {
            pool_id: 3,
            token_in: "c:uatom".to_string(),
            token_out: "y:uatom:30Sep2024".to_string(),
            amount: Some("1200".to_string()),
        }],
        "950",
        "c:uatom",
    );

    mock_batch_swap_given_in(
        &mut querier,
        vec![
            SwapStep {
                pool_id: 1,
                token_in: "ibc/uosmo".to_string(),
                token_out: "ibc/uusdc".to_string(),
                amount: Some("3000".to_string()),
            },
            SwapStep {
                pool_id: 2,
                token_in: "ibc/uusdc".to_string(),
                token_out: "ibc/uatom".to_string(),
                amount: None,
            },
            SwapStep {
                pool_id: 4,
                token_in: "ibc/uatom".to_string(),
                token_out: "c:uatom".to_string(),
                amount: None,
            },
            SwapStep {
                pool_id: 3,
                token_in: "c:uatom".to_string(),
                token_out: "y:uatom:30Sep2024".to_string(),
                amount: None,
            }
        ],
        "1705",
        "ibc/y:uatom:30Sep2024",
    );

    mock_spot_price(&mut querier, 1, "ibc/uosmo", "ibc/uusdc", "0.25");
    mock_spot_price(&mut querier, 2, "ibc/uusdc", "ibc/uatom", "2");
    mock_spot_price(
        &mut querier,
        3,
        "c:uatom",
        "y:uatom:30Sep2024",
        "1.263157895",
    );
    mock_spot_price(&mut querier, 4, "ibc/uatom", "c:uatom", "0.9");

    querier
}

fn mock_stake_given_in(
    querier: &mut MockQuerier,
    host_chain: &str,
    channel: &str,
    amount_in: &str,
    amount_out: &str,
) {
    querier.mock_query(
        QuerySimulateStakeRequest {
            host_chain: host_chain.to_string(),
            transfer_channel: channel.to_string(),
            amount_in: Some(amount_in.to_string()),
            amount_out: None,
        }
        .into(),
        &QuerySimulateStakeResponse {
            amount_in: None,
            amount_out: Some(CosmosCoin {
                amount: amount_out.to_string(),
                denom: format!("c:{}", host_chain),
            }),
            fee_amount: Some(CosmosCoin {
                amount: "0".to_string(),
                denom: format!("ibc/{}", host_chain),
            }),
        },
    );
}

fn mock_stake_given_out(
    querier: &mut MockQuerier,
    host_chain: &str,
    channel: &str,
    amount_out: &str,
    amount_in: &str,
) {
    querier.mock_query(
        QuerySimulateStakeRequest {
            host_chain: host_chain.to_string(),
            transfer_channel: channel.to_string(),
            amount_in: None,
            amount_out: Some(amount_out.to_string()),
        }
        .into(),
        &QuerySimulateStakeResponse {
            amount_in: Some(CosmosCoin {
                amount: amount_in.to_string(),
                denom: format!("ibc/{}", host_chain),
            }),
            amount_out: None,
            fee_amount: Some(CosmosCoin {
                amount: "0".to_string(),
                denom: format!("ibc/{}", host_chain),
            }),
        },
    );
}

fn mock_batch_swap_given_in(
    querier: &mut MockQuerier,
    swap_steps: Vec<SwapStep>,
    out_amount: &str,
    out_denom: &str,
) {
    querier.mock_query(
        QuerySimulateBatchSwapRequest {
            swap_type: SwapType::GivenIn.into(),
            steps: swap_steps,
        }
        .into(),
        &QuerySimulateBatchSwapResponse {
            amounts_out: vec![CosmosCoin {
                amount: out_amount.to_string(),
                denom: out_denom.to_string(),
            }],
            amounts_in: vec![],
            swap_fee: vec![],
            join_exit_protocol_fee: vec![],
            swap_protocol_fee: vec![],
        },
    );
}

fn mock_batch_swap_given_out(
    querier: &mut MockQuerier,
    swap_steps: Vec<SwapStep>,
    in_amount: &str,
    in_denom: &str,
) {
    querier.mock_query(
        QuerySimulateBatchSwapRequest {
            swap_type: SwapType::GivenOut.into(),
            steps: swap_steps,
        }
        .into(),
        &QuerySimulateBatchSwapResponse {
            amounts_in: vec![CosmosCoin {
                amount: in_amount.to_string(),
                denom: in_denom.to_string(),
            }],
            amounts_out: vec![],
            swap_fee: vec![],
            join_exit_protocol_fee: vec![],
            swap_protocol_fee: vec![],
        },
    );
}

fn mock_spot_price(
    querier: &mut MockQuerier,
    pool_id: u64,
    token_in: &str,
    token_out: &str,
    spot_price: &str,
) {
    querier.mock_query(
        QuerySpotPriceRequest {
            pool_id,
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            apply_fee: false,
        }
        .into(),
        &QuerySpotPriceResponse {
            spot_price: spot_price.to_string(),
        },
    );
}
