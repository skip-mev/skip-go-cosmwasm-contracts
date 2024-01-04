use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env},
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Reply, StdError, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use skip::asset::Asset;
use skip_api_entry_point::{reply::RecoverTempStorage, state::RECOVER_TEMP_STORAGE};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Native Asset Sent On Error
    - Native Asset Not Sent On Success
    - Cw20 Asset Sent On Error
    - Cw20 Asset Not Sent On Success
    - Native And Cw20 Asset Sent On Error

Expect Error
    - Verify error on invalid reply id

*/

// Define test parameters
struct Params {
    reply: Reply,
    expected_error_string: String,
    storage: Option<RecoverTempStorage>,
    expected_messages: Vec<SubMsg>,
}

//  Test reply
#[test_case(
    Params {
        reply: Reply {
            id: 1,
            result: SubMsgResult::Err("Anything".to_string()),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            assets: vec![Asset::Native(Coin::new(1_000_000, "osmo"))],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: Addr::unchecked("recovery_addr").to_string(),
            amount: vec![Coin::new(1_000_000, "osmo")],
        }))],
    };
    "Native Asset Funds Sent On Error"
)]
#[test_case(
    Params {
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            assets: vec![Asset::Native(Coin::new(1_000_000, "osmo"))],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![],
    };
    "Native Asset Funds Not Sent On Success"
)]
#[test_case(
    Params {
        reply: Reply {
            id: 1,
            result: SubMsgResult::Err("Anything".to_string()),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            assets: vec![Asset::Cw20(Cw20Coin{
                address: "neutron123".to_string(),
                amount: Uint128::new(1_000_000),
            })],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "neutron123".to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "recovery_addr".to_string(),
                amount: Uint128::new(1_000_000),
            }).unwrap(),
            funds: vec![],
        }))],
    };
    "Cw20 Asset Sent On Error"
)]
#[test_case(
    Params {
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            assets: vec![Asset::Cw20(Cw20Coin{
                address: "neutron123".to_string(),
                amount: Uint128::new(1_000_000),
            })],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![],
    };
    "Cw20 Asset Not Sent On Success"
)]
#[test_case(
    Params {
        reply: Reply {
            id: 1,
            result: SubMsgResult::Err("Anything".to_string()),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            assets: vec![
                Asset::Native(Coin::new(1_000_000, "osmo")),
            Asset::Cw20(Cw20Coin{
                address: "neutron123".to_string(),
                amount: Uint128::new(1_000_000),
            })],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: Addr::unchecked("recovery_addr").to_string(),
                amount: vec![Coin::new(1_000_000, "osmo")],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "neutron123".to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "recovery_addr".to_string(),
                    amount: Uint128::new(1_000_000),
                }).unwrap(),
                funds: vec![],
        }))],
    };
    "Native And Cw20 Asset Sent On Error"
)]
#[test_case(
    Params {
        reply: Reply {
            id: 2,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
        expected_error_string: "Reply id: 2 not valid".to_string(),
        storage: None,
        expected_messages: vec![],
    };
    "Verify error on invalid reply id"
)]
fn test_reply(params: Params) {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock env
    let env = mock_env();

    // Update storage
    if let Some(recover_temp_storage) = params.storage.clone() {
        RECOVER_TEMP_STORAGE
            .save(deps.as_mut().storage, &recover_temp_storage)
            .unwrap();
    }

    // Call reply with the given test parameters
    let res = skip_api_entry_point::contract::reply(deps.as_mut(), env, params.reply);

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error_string.is_empty(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error_string
            );

            assert_eq!(res.messages, params.expected_messages);

            // Verify the in progress recover temp storage was removed from storage
            match RECOVER_TEMP_STORAGE.load(&deps.storage) {
                Ok(recover_temp_storage) => {
                    panic!(
                        "expected in progress recover_temp_storage to be removed: {:?}",
                        recover_temp_storage
                    )
                }
                Err(err) => assert!(
                    matches!(err, StdError::NotFound { .. }),
                    "unexpected error: {:?}",
                    err
                ),
            };
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
}
