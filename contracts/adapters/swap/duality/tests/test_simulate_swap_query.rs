use cosmwasm_std::{
    testing::{mock_env, MockApi, MockStorage},
    from_json, to_json_binary,
};
use cosmwasm_std::{Addr, Coin,OwnedDeps};
use skip::swap::{QueryMsg, SimulateSwapExactAssetInResponse, SimulateSwapExactAssetOutResponse, SwapOperation};
use skip_go_swap_adapter_duality::{error::ContractResult, state::ENTRY_POINT_CONTRACT_ADDRESS};
use neutron_sdk::stargate::dex::types::{
    MHRoute, MultiHopSwapResponse, PlaceLimitOrderResponse, SimulateMultiHopSwapResponse, SimulatePlaceLimitOrderResponse,
    PoolReserves, PoolReservesKey, TradePairID, AllTickLiquidityResponse, TickLiquidity
};
use std::marker::PhantomData;
use skip::asset::Asset;
mod mock;
use mock::MockQuerier;

use test_case::test_case;


struct ExactAssetInParams {
    coin: Coin,
    swap_operations: Vec<SwapOperation>,
    expected_result: SimulateSwapExactAssetInResponse,
}


#[test_case(
    ExactAssetInParams {
        coin: Coin::new(100, "os"),
        swap_operations: vec![SwapOperation {
            pool: "1".to_string(),
            denom_in: "os".to_string(),
            denom_out: "uatom".to_string(),
            interface: None,
        }],
        expected_result: SimulateSwapExactAssetInResponse {
            asset_out: Coin::new(100, "uatom").into(),
            spot_price: None,
        },
    }; "Succesful SwapExactAssetIn")]
    #[test_case(
        ExactAssetInParams {
            coin: Coin::new(1000000000000000000000, "os"),
            swap_operations: vec![SwapOperation {
                pool: "1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }],
            expected_result: SimulateSwapExactAssetInResponse {
                asset_out: Coin::new(1000000000000000000000, "uatom").into(),
                spot_price: None,
        },
    }; "Succesful SwapExactAssetIn with large amount")]

fn test_simulate_swap_exact_asset_in(params: ExactAssetInParams) -> ContractResult<()> {
    // Create mock dependencies with custom querier
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::default(),
        custom_query_type: PhantomData,
    };

    // Create mock env
    let env = mock_env();

    // Store required contract addresses
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;

   

    let response = SimulateMultiHopSwapResponse {
        resp: MultiHopSwapResponse {
            coin_out: Coin::new(params.coin.amount.into(), "coin"),
            route: MHRoute {
                hops: params.swap_operations.iter().map(|op| op.denom_out.clone()).collect(),
            },
            dust: vec![],
        }
    };

    deps.querier.mock_stargate_response(
        ("MultiHopSwap".into() , to_json_binary(&response).unwrap()),
    );

    // Call simulate_swap_exact_asset_in
    let res = skip_go_swap_adapter_duality::contract::query(
        deps.as_ref(),
        env,
        QueryMsg::SimulateSwapExactAssetInWithMetadata {
            asset_in: params.coin.into(),
            swap_operations: params.swap_operations,
            include_spot_price: false,
        },
    )?;

    // Deserialize response
    let simulation_response: SimulateSwapExactAssetInResponse = from_json(res)?;

    // Assert response matches expected result
    assert_eq!(simulation_response, params.expected_result);

    Ok(())
}


struct ExactAssetOutParams {
    coin: Coin,
    swap_operations: Vec<SwapOperation>,
    expected_result: SimulateSwapExactAssetOutResponse,
}
#[test_case(
    ExactAssetOutParams {
        coin: Coin::new(100, "uatom"),
        swap_operations: vec![SwapOperation {
            pool: "1".to_string(),
            denom_in: "os".to_string(),
            denom_out: "uatom".to_string(),
            interface: None,
        }],
        expected_result: SimulateSwapExactAssetOutResponse {
            asset_in: Coin::new(100, "os").into(),
            spot_price: None,
        },
    }; "Succesful SwapExactAssetOut")]
    #[test_case(
        ExactAssetOutParams {
            coin: Coin::new(1000000000000000000000, "uatom"),
            swap_operations: vec![SwapOperation {
                pool: "1".to_string(),
                denom_in: "os".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            }],
            expected_result: SimulateSwapExactAssetOutResponse {
                asset_in: Coin::new(1000000000000000000000, "os").into(),
                spot_price: None,
        },
    }; "Succesful SwapExactAssetOut with large amount")]
fn test_simulate_swap_exact_asset_out(params: ExactAssetOutParams) -> ContractResult<()> {
    // Create mock dependencies with custom querier
    let mut deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::default(),
        custom_query_type: PhantomData,
    };

    // Create mock env
    let env = mock_env();

    // Store required contract addresses
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;

    let amount_in = params.coin.amount;

    let asset_out: Asset = params.coin.into();

    let response = SimulatePlaceLimitOrderResponse {
        resp: PlaceLimitOrderResponse {
            trancheKey: "".to_string(),
            coin_in: Some(Coin::new(amount_in.into(), "someDenom")),
            taker_coin_out: Some(Coin::new(amount_in.into(), asset_out.denom())),
            taker_coin_in: Some(Coin::new(amount_in.into(), "someDenom")),
        }
    };

    deps.querier.mock_stargate_response(
        ("PlaceLimitOrder".into() , to_json_binary(&response).unwrap()),
    );

    let response = AllTickLiquidityResponse {
        tick_liquidity: vec![
            TickLiquidity::PoolReserves(PoolReserves {
                key: PoolReservesKey {
                    trade_pair_id: TradePairID {
                            maker_denom: "os".to_string(),
                        taker_denom: "uatom".to_string(),
                    },
                    tick_index_taker_to_maker: 1.into(),
                    fee: None,
                },
                reserves_maker_denom: 100.into(),
                price_taker_to_maker: "1".to_string(),
                price_opposite_taker_to_maker: "1".to_string(),
            }),
        ],
        pagination: None,
    };

    deps.querier.mock_stargate_response(
        ("TickLiquidityAll".into() , to_json_binary(&response).unwrap()),
    );


    // Call simulate_swap_exact_asset_out
    let res = skip_go_swap_adapter_duality::contract::query(
        deps.as_ref(),
        env,
        QueryMsg::SimulateSwapExactAssetOutWithMetadata {
            asset_out: asset_out.clone(),
            swap_operations: params.swap_operations,
            include_spot_price: false,
        },
    )?;

    // Deserialize response
    let simulation_response: SimulateSwapExactAssetOutResponse = from_json(res)?;

    // Assert response matches expected result
    assert_eq!(simulation_response, params.expected_result);

    Ok(())
}
