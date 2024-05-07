use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env, mock_info},
    to_json_binary, Addr, Coin, CosmosMsg, ReplyOn, SubMsg, Timestamp, Uint128, WasmMsg,
};
use cw20::Cw20Coin;
use skip::{
    asset::Asset,
    entry_point::{Action, Affiliate, ExecuteMsg},
    swap::{Swap, SwapExactAssetIn, SwapOperation},
};
use skip_api_entry_point::{error::ContractError, state::RECOVER_TEMP_STORAGE};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Happy Path Single Coin
    - Happy Path Multiple Coins
    - Happy Path Cw20 Asset
    - Sent Asset Not Given With Valid One Coin
    - Sent Asset Not Given With Invalid One Coin

    // Note: The following test case is an invalid call to the contract
    // showing that under the circumstance both coins and a Cw20 token
    // is sent to the contract, the contract will recover all assets.
    - Happy Path Multiple Coins And Cw20 Asset

*/

// Define test parameters
struct Params {
    info_funds: Vec<Coin>,
    sent_asset: Option<Asset>,
    user_swap: Swap,
    min_asset: Asset,
    timeout_timestamp: u64,
    post_swap_action: Action,
    affiliates: Vec<Affiliate>,
    expected_assets: Vec<Asset>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap_and_action_with_recover
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "untrn")],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
                interface: None,
            }],
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_assets: vec![Asset::Native(Coin::new(1_000_000, "untrn"))],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                    sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
                    user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }],
                    }),
                    min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                    timeout_timestamp: 101,
                    post_swap_action: Action::Transfer {
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
    "Happy Path Single Coin")]
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "untrn"), Coin::new(1_000_000, "osmo")],
        sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
        user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
                interface: None,
            }],
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_assets: vec![Asset::Native(Coin::new(1_000_000, "untrn")), Asset::Native(Coin::new(1_000_000, "osmo"))],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                    sent_asset: Some(Asset::Native(Coin::new(1_000_000, "untrn"))),
                    user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }],
                    }),
                    min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                    timeout_timestamp: 101,
                    post_swap_action: Action::Transfer {
                        to_address: "to_address".to_string(),
                    },
                    affiliates: vec![],
                })
                .unwrap(),
                funds: vec![Coin::new(1000000, "untrn"), Coin::new(1000000, "osmo")],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }],
        expected_error: None,
    };
    "Happy Path Multiple Coins")]
#[test_case(
    Params {
        info_funds: vec![],
        sent_asset: Some(Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })),
        user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "neutron123".to_string(),
                denom_out: "osmo".to_string(),
                interface: None,
            }],
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_assets: vec![Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                    sent_asset: Some(Asset::Cw20(Cw20Coin{
                        address: "neutron123".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    })),
                    user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "neutron123".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }],
                    }),
                    min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                    timeout_timestamp: 101,
                    post_swap_action: Action::Transfer {
                        to_address: "to_address".to_string(),
                    },
                    affiliates: vec![],
                })
                .unwrap(),
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }],
        expected_error: None,
    };
    "Happy Path Cw20 Asset")]
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "untrn")],
        sent_asset: None,
        user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
                interface: None,
            }],
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_assets: vec![Asset::Native(Coin::new(1_000_000, "untrn"))],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                    sent_asset: None,
                    user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }],
                    }),
                    min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                    timeout_timestamp: 101,
                    post_swap_action: Action::Transfer {
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
    "Sent Asset Not Given With Valid One Coin")]
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "untrn"), Coin::new(1_000_000, "osmo")],
        sent_asset: None,
        user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
                interface: None,
            }],
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_assets: vec![Asset::Native(Coin::new(1_000_000, "untrn")), Asset::Native(Coin::new(1_000_000, "osmo"))],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                    sent_asset: None,
                    user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }],
                    }),
                    min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                    timeout_timestamp: 101,
                    post_swap_action: Action::Transfer {
                        to_address: "to_address".to_string(),
                    },
                    affiliates: vec![],
                })
                .unwrap(),
                funds: vec![Coin::new(1000000, "untrn"), Coin::new(1000000, "osmo")],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }],
        expected_error: None,
    };
    "Sent Asset Not Given With Invalid One Coin")]
#[test_case(
    Params {
        info_funds: vec![Coin::new(1_000_000, "untrn"), Coin::new(1_000_000, "osmo")],
        sent_asset: Some(Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })),
        user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
            swap_venue_name: "swap_venue_name".to_string(),
            operations: vec![SwapOperation {
                pool: "pool".to_string(),
                denom_in: "untrn".to_string(),
                denom_out: "osmo".to_string(),
                interface: None,
            }],
        }),
        min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
        timeout_timestamp: 101,
        post_swap_action: Action::Transfer {
            to_address: "to_address".to_string(),
        },
        affiliates: vec![],
        expected_assets: vec![Asset::Native(Coin::new(1_000_000, "untrn")), Asset::Native(Coin::new(1_000_000, "osmo")), Asset::Cw20(Cw20Coin{
            address: "neutron123".to_string(),
            amount: Uint128::from(1_000_000u128),
        })],
        expected_messages: vec![SubMsg {
            id: 1,
            msg: CosmosMsg::from(WasmMsg::Execute {
                contract_addr: "entry_point".to_string(),
                msg: to_json_binary(&ExecuteMsg::SwapAndAction {
                    sent_asset: Some(Asset::Cw20(Cw20Coin{
                        address: "neutron123".to_string(),
                        amount: Uint128::from(1_000_000u128),
                    })),
                    user_swap: Swap::SwapExactAssetIn(SwapExactAssetIn {
                        swap_venue_name: "swap_venue_name".to_string(),
                        operations: vec![SwapOperation {
                            pool: "pool".to_string(),
                            denom_in: "untrn".to_string(),
                            denom_out: "osmo".to_string(),
                            interface: None,
                        }],
                    }),
                    min_asset: Asset::Native(Coin::new(1_000_000, "osmo")),
                    timeout_timestamp: 101,
                    post_swap_action: Action::Transfer {
                        to_address: "to_address".to_string(),
                    },
                    affiliates: vec![],
                })
                .unwrap(),
                funds: vec![Coin::new(1000000, "untrn"), Coin::new(1000000, "osmo")],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }],
        expected_error: None,
    };
    "Happy Path Multiple Coins And Cw20 Asset")]
fn test_execute_swap_and_action_with_recover(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock env with parameters that make testing easier
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("entry_point");
    env.block.time = Timestamp::from_nanos(100);

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info("swapper", info_funds);

    let recovery_addr = Addr::unchecked("recovery_address");
    // Call execute_swap_and_action with the given test case params
    let res = skip_api_entry_point::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::SwapAndActionWithRecover {
            sent_asset: params.sent_asset,
            user_swap: params.user_swap,
            min_asset: params.min_asset,
            timeout_timestamp: params.timeout_timestamp,
            post_swap_action: params.post_swap_action,
            affiliates: params.affiliates,
            recovery_addr: recovery_addr.clone(),
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

            // Assert the recover temp storage is correct
            let recover_temp_storage = RECOVER_TEMP_STORAGE.load(&deps.storage).unwrap();
            assert_eq!(recover_temp_storage.recovery_addr, recovery_addr);
            assert_eq!(recover_temp_storage.assets, params.expected_assets);
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
