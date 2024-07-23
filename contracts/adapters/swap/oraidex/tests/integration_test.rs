use std::str::FromStr;

use cosmwasm_std::{to_json_binary, Addr, Coin, Decimal, Decimal256, Uint128};
use cw_multi_test::{addons::MockApiBech32, App, BankKeeper, Executor};
use dexter::{
    asset::{Asset as DexterAsset, AssetInfo as DexterAssetInfo},
    vault::{self, FeeInfo, NativeAssetPrecisionInfo},
};
use dexter_stable_pool::state::{AssetScalingFactor, StablePoolParams};
use skip::{asset::Asset, swap::SwapOperation};
use utils::{
    instantiate_dexter_contracts_and_pools, instantiate_dexter_swap_adapter_contract,
    DexterInstantiateResponse,
};

mod utils;

pub struct SetupResponse {
    pub app: App<BankKeeper, MockApiBech32>,
    pub skip_swap_adapter_contract: Addr,
    pub dexter_init_response: DexterInstantiateResponse,
}

pub fn setup() -> SetupResponse {
    let mock_api = MockApiBech32::new("persistence");
    let owner = mock_api.addr_make("owner");

    let coins = vec![
        Coin::new(100_000_000_000, "uxprt"),
        Coin::new(100_000_000_000, "stk/uxprt"),
        Coin::new(100_000_000_000, "stk/uatom"),
    ];

    let mut app = utils::mock_app(owner.clone(), coins);

    let fee_info = FeeInfo {
        total_fee_bps: 30,
        protocol_fee_percent: 30,
    };

    let pool_instantiate_msgs = vec![
        dexter::vault::ExecuteMsg::CreatePoolInstance {
            pool_type: vault::PoolType::StableSwap {},
            asset_infos: vec![
                DexterAssetInfo::native_token("uxprt".to_string()),
                DexterAssetInfo::native_token("stk/uxprt".to_string()),
            ],
            native_asset_precisions: vec![
                NativeAssetPrecisionInfo {
                    precision: 6,
                    denom: "uxprt".to_string(),
                },
                NativeAssetPrecisionInfo {
                    precision: 6,
                    denom: "stk/uxprt".to_string(),
                },
            ],
            fee_info: None,
            init_params: Some(
                to_json_binary(&StablePoolParams {
                    amp: 50,
                    supports_scaling_factors_update: false,
                    scaling_factors: vec![
                        AssetScalingFactor {
                            asset_info: DexterAssetInfo::native_token("uxprt".to_string()),
                            scaling_factor: Decimal256::one(),
                        },
                        AssetScalingFactor {
                            asset_info: DexterAssetInfo::native_token("stk/uxprt".to_string()),
                            scaling_factor: Decimal256::one(),
                        },
                    ],
                    scaling_factor_manager: None,
                })
                .unwrap(),
            ),
        },
        dexter::vault::ExecuteMsg::CreatePoolInstance {
            pool_type: vault::PoolType::Weighted {},
            asset_infos: vec![
                DexterAssetInfo::native_token("stk/uxprt".to_string()),
                DexterAssetInfo::native_token("stk/uatom".to_string()),
            ],
            native_asset_precisions: vec![
                NativeAssetPrecisionInfo {
                    precision: 6,
                    denom: "stk/uxprt".to_string(),
                },
                NativeAssetPrecisionInfo {
                    precision: 6,
                    denom: "stk/uatom".to_string(),
                },
            ],
            fee_info: None,
            init_params: Some(
                to_json_binary(&dexter_weighted_pool::state::WeightedParams {
                    weights: vec![
                        DexterAsset::new(
                            DexterAssetInfo::native_token("stk/uxprt".to_string()),
                            Uint128::from(1u128),
                        ),
                        DexterAsset::new(
                            DexterAssetInfo::native_token("stk/uatom".to_string()),
                            Uint128::from(1u128),
                        ),
                    ],
                    exit_fee: None,
                })
                .unwrap(),
            ),
        },
    ];

    let dexter_init_response =
        instantiate_dexter_contracts_and_pools(&mut app, &owner, fee_info, pool_instantiate_msgs);

    let skip_swap_adapter_contract = instantiate_dexter_swap_adapter_contract(
        &mut app,
        &owner,
        &mock_api.addr_make("entry_point"),
        &dexter_init_response.dexter_vault_addr,
        &dexter_init_response.dexter_router_addr,
    );

    let join_pool_msg_1 = dexter::vault::ExecuteMsg::JoinPool {
        pool_id: Uint128::from(1u128),
        assets: Some(vec![
            DexterAsset::new(
                DexterAssetInfo::native_token("stk/uxprt".to_string()),
                Uint128::from(100_000_000u128),
            ),
            DexterAsset::new(
                DexterAssetInfo::native_token("uxprt".to_string()),
                Uint128::from(100_000_000u128),
            ),
        ]),
        recipient: None,
        min_lp_to_receive: None,
        auto_stake: None,
    };

    app.execute_contract(
        owner.clone(),
        dexter_init_response.dexter_vault_addr.clone(),
        &join_pool_msg_1,
        &[
            Coin::new(100_000_000u128, "stk/uxprt"),
            Coin::new(100_000_000u128, "uxprt"),
        ],
    )
    .unwrap();

    let join_pool_msg_2 = dexter::vault::ExecuteMsg::JoinPool {
        pool_id: Uint128::from(2u128),
        assets: Some(vec![
            DexterAsset::new(
                DexterAssetInfo::native_token("stk/uatom".to_string()),
                Uint128::from(100_000_000u128),
            ),
            DexterAsset::new(
                DexterAssetInfo::native_token("stk/uxprt".to_string()),
                Uint128::from(100_000_000u128),
            ),
        ]),
        recipient: None,
        min_lp_to_receive: None,
        auto_stake: None,
    };

    app.execute_contract(
        owner.clone(),
        dexter_init_response.dexter_vault_addr.clone(),
        &join_pool_msg_2,
        &[
            Coin::new(100_000_000u128, "stk/uatom"),
            Coin::new(100_000_000u128, "stk/uxprt"),
        ],
    )
    .unwrap();

    SetupResponse {
        app,
        skip_swap_adapter_contract,
        dexter_init_response,
    }
}

#[test]
pub fn test_swap_simulation() {
    let SetupResponse {
        app,
        skip_swap_adapter_contract,
        dexter_init_response: _,
    } = setup();

    // simulate swap of 1 uxprt to stk/uxprt via 1 pool
    let swap_simulation_msg = skip::swap::QueryMsg::SimulateSwapExactAssetInWithMetadata {
        asset_in: skip::asset::Asset::Native(Coin {
            denom: "uxprt".to_string(),
            amount: Uint128::from(1_000_000u128),
        }),
        swap_operations: vec![SwapOperation {
            pool: "1".to_string(),
            denom_in: "uxprt".to_string(),
            denom_out: "stk/uxprt".to_string(),
            interface: None,
        }],
        include_spot_price: true,
    };

    let simulation_result: skip::swap::SimulateSwapExactAssetInResponse = app
        .wrap()
        .query_wasm_smart(skip_swap_adapter_contract.clone(), &swap_simulation_msg)
        .unwrap();

    // assert output
    assert_eq!(
        simulation_result.asset_out,
        Asset::Native(Coin {
            denom: "stk/uxprt".to_string(),
            amount: Uint128::from(996806u128)
        })
    );

    // assert spot price
    assert_eq!(
        simulation_result.spot_price.unwrap(),
        Decimal::from_str("0.996999805096313").unwrap()
    );

    // simulate swap of 1 uxprt to stk/uatom via 2 pools
    let swap_simulation_msg = skip::swap::QueryMsg::SimulateSwapExactAssetInWithMetadata {
        asset_in: skip::asset::Asset::Native(Coin {
            denom: "uxprt".to_string(),
            amount: Uint128::from(1_000_000u128),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uxprt".to_string(),
                denom_out: "stk/uxprt".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "stk/uxprt".to_string(),
                denom_out: "stk/uatom".to_string(),
                interface: None,
            },
        ],
        include_spot_price: true,
    };

    let simulation_result: skip::swap::SimulateSwapExactAssetInResponse = app
        .wrap()
        .query_wasm_smart(skip_swap_adapter_contract.clone(), &swap_simulation_msg)
        .unwrap();

    // assert output
    assert_eq!(
        simulation_result.asset_out,
        Asset::Native(Coin {
            denom: "stk/uatom".to_string(),
            amount: Uint128::from(984036u128)
        })
    );

    // assert spot price
    assert_eq!(
        simulation_result.spot_price.unwrap(),
        Decimal::from_str("0.994008805681024061").unwrap()
    );

    // do a reverse simulation and validate the output
    let swap_simulation_msg = skip::swap::QueryMsg::SimulateSwapExactAssetOutWithMetadata {
        asset_out: skip::asset::Asset::Native(Coin {
            denom: "stk/uatom".to_string(),
            amount: Uint128::from(1_000_000u128),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uxprt".to_string(),
                denom_out: "stk/uxprt".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "stk/uxprt".to_string(),
                denom_out: "stk/uatom".to_string(),
                interface: None,
            },
        ],
        include_spot_price: true,
    };

    let simulation_result: skip::swap::SimulateSwapExactAssetOutResponse = app
        .wrap()
        .query_wasm_smart(skip_swap_adapter_contract.clone(), &swap_simulation_msg)
        .unwrap();

    // assert output
    assert_eq!(
        simulation_result.asset_in,
        Asset::Native(Coin {
            denom: "uxprt".to_string(),
            amount: Uint128::from(1016390u128)
        })
    );

    // assert spot price. point to note that this spot price is same as in the previous case meaning that the spot price across a route is consistent in both simulations
    assert_eq!(
        simulation_result.spot_price.unwrap(),
        Decimal::from_str("0.994008805681024061").unwrap()
    );

    // Let's simulate a swap with a route that doesn't exist
    let swap_simulation_msg = skip::swap::QueryMsg::SimulateSwapExactAssetInWithMetadata {
        asset_in: skip::asset::Asset::Native(Coin {
            denom: "uxprt".to_string(),
            amount: Uint128::from(1_000_000u128),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uxprt".to_string(),
                denom_out: "stk/uxprt".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "stk/uxprt".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
        ],
        include_spot_price: true,
    };

    let simulation_result: Result<skip::swap::SimulateSwapExactAssetInResponse, _> = app
        .wrap()
        .query_wasm_smart(skip_swap_adapter_contract.clone(), &swap_simulation_msg);

    // error type is weird, for now we'll just check if there's an error
    assert!(simulation_result.is_err());

    // Let's try to send an invalid swap operation
    let swap_simulation_msg = skip::swap::QueryMsg::SimulateSwapExactAssetInWithMetadata {
        asset_in: skip::asset::Asset::Native(Coin {
            denom: "uxprt".to_string(),
            amount: Uint128::from(1_000_000u128),
        }),
        swap_operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uxprt".to_string(),
                denom_out: "stk/uxprt".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "stk/uxprt".to_string(),
                denom_out: "stk/uatom".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "3".to_string(),
                denom_in: "stk/uatom".to_string(),
                denom_out: "uatom".to_string(),
                interface: None,
            },
        ],
        include_spot_price: true,
    };

    let simulation_result: Result<skip::swap::SimulateSwapExactAssetInResponse, _> = app
        .wrap()
        .query_wasm_smart(skip_swap_adapter_contract.clone(), &swap_simulation_msg);

    // error type is weird, for now we'll just check if there's an error
    assert!(simulation_result.is_err());
}

#[test]
pub fn test_swap() {
    let SetupResponse {
        mut app,
        skip_swap_adapter_contract,
        dexter_init_response: _,
    } = setup();

    // simulate swap of 1 uxprt to stk/uxprt via 1 pool
    let swap_msg = skip::swap::ExecuteMsg::Swap {
        operations: vec![SwapOperation {
            pool: "1".to_string(),
            denom_in: "uxprt".to_string(),
            denom_out: "stk/uxprt".to_string(),
            interface: None,
        }],
    };

    // create a new user and fund them with some uxprt
    let entrypoint_dummy = app.api().addr_make("entry_point");
    app.send_tokens(
        app.api().addr_make("owner"),
        entrypoint_dummy.clone(),
        &[Coin::new(2_000_000u128, "uxprt")],
    )
    .unwrap();

    // execute the swap without funds
    let res = app.execute_contract(
        entrypoint_dummy.clone(),
        skip_swap_adapter_contract.clone(),
        &swap_msg,
        &[],
    );

    // swap without funds should fail
    assert!(res.is_err());
    assert_eq!(res.err().unwrap().root_cause().to_string(), "No funds sent");

    // execute the swap
    app.execute_contract(
        entrypoint_dummy.clone(),
        skip_swap_adapter_contract.clone(),
        &swap_msg,
        &[Coin::new(1_000_000u128, "uxprt")],
    )
    .unwrap();

    // assert that the user has received the output asset
    let user_balance: Uint128 = app
        .wrap()
        .query_balance(entrypoint_dummy.clone(), "stk/uxprt")
        .unwrap()
        .amount;
    assert_eq!(user_balance, Uint128::from(996806u128));

    // perform a swap with multiple pools
    let swap_msg = skip::swap::ExecuteMsg::Swap {
        operations: vec![
            SwapOperation {
                pool: "1".to_string(),
                denom_in: "uxprt".to_string(),
                denom_out: "stk/uxprt".to_string(),
                interface: None,
            },
            SwapOperation {
                pool: "2".to_string(),
                denom_in: "stk/uxprt".to_string(),
                denom_out: "stk/uatom".to_string(),
                interface: None,
            },
        ],
    };

    // execute the swap
    app.execute_contract(
        entrypoint_dummy.clone(),
        skip_swap_adapter_contract.clone(),
        &swap_msg,
        &[Coin::new(1_000_000u128, "uxprt")],
    )
    .unwrap();

    // assert that the user has received the output asset
    let user_balance: Uint128 = app
        .wrap()
        .query_balance(entrypoint_dummy.clone(), "stk/uatom")
        .unwrap()
        .amount;
    assert_eq!(user_balance, Uint128::from(983654u128));
}
