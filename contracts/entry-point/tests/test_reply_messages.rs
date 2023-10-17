use cosmwasm_std::testing::{mock_dependencies_with_balances, mock_env};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Reply, StdError, SubMsg, SubMsgResponse,
    SubMsgResult,
};
use skip::entry_point::Action::BankSend;
use skip_api_entry_point::error::ContractError::Timeout;
use skip_api_entry_point::error::ContractResult;
use skip_api_entry_point::reply::RecoverTempStorage;
use skip_api_entry_point::state::RECOVER_TEMP_STORAGE;

pub struct Params {
    pub funds: Vec<Coin>,
    pub reply: Reply,
    pub expected_error_string: String,
    pub storage: Option<RecoverTempStorage>,
    pub expected_messages: Vec<SubMsg>,
}

//  Helper function to test all replies
pub fn test_reply(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[(
        "entry_point",
        &[Coin::new(1_000_000, "osmo"), Coin::new(1_000_000, "untrn")],
    )]);

    // Create mock env
    let env = mock_env();

    // Update storage
    if let Some(swap_action) = params.storage.clone() {
        RECOVER_TEMP_STORAGE.save(deps.as_mut().storage, &swap_action)?;
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

            // Verify the in progress swap and action message was removed from storage
            match RECOVER_TEMP_STORAGE.load(&deps.storage) {
                Ok(swap_and_action_request) => {
                    panic!(
                        "expected in progress ibc transfer to be removed: {:?}",
                        swap_and_action_request
                    )
                }
                Err(err) => assert_eq!(
                    err,
                    StdError::NotFound {
                        kind: "skip_api_entry_point::reply::RecoverTempStorage".to_string(),
                    }
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

    Ok(())
}

#[test]
pub fn verify_funds_sent_on_slippage_error() {
    let recovery_addr = Addr::unchecked("recovery_addr").to_string();
    let bank_msg = BankMsg::Send {
        to_address: recovery_addr,
        amount: vec![Coin::new(1_000_000, "osmo")],
    };

    let sub_msg = SubMsg::new(CosmosMsg::Bank(bank_msg));

    let params = Params {
        funds: vec![],
        reply: Reply {
            id: 1,
            result: SubMsgResult::Err("Slippage tolerance exceeded".to_string()),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            funds: vec![Coin::new(1_000_000, "osmo")],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![sub_msg],
    };

    test_reply(params).unwrap();
}

#[test]
pub fn verify_funds_sent_on_timeout_error() {
    let recovery_addr = Addr::unchecked("recovery_addr").to_string();
    let bank_msg = BankMsg::Send {
        to_address: recovery_addr,
        amount: vec![Coin::new(1_000_000, "osmo")],
    };

    let sub_msg = SubMsg::new(CosmosMsg::Bank(bank_msg));

    let params = Params {
        funds: vec![],
        reply: Reply {
            id: 1,
            result: SubMsgResult::Err((Timeout).to_string()),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            funds: vec![Coin::new(1_000_000, "osmo")],
            recovery_addr: Addr::unchecked("recovery_addr"),
        }),
        expected_messages: vec![sub_msg],
    };

    test_reply(params).unwrap();
}

#[test]
pub fn invalid_reply_id_error() {
    let recovery_addr = Addr::unchecked("recovery_addr");
    let params = Params {
        funds: vec![],
        reply: Reply {
            id: 2,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(
                    to_binary(&BankSend {
                        to_address: recovery_addr.to_string(),
                    })
                    .unwrap(),
                ),
            }),
        },
        expected_error_string: "Reply id: 2 not valid".to_string(),
        storage: None,
        expected_messages: vec![],
    };

    test_reply(params).unwrap();
}

#[test]
pub fn success_case_no_funds_sent() {
    let recovery_addr = Addr::unchecked("recovery_addr");

    let params = Params {
        funds: vec![],
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
        expected_error_string: "".to_string(),
        storage: Some(RecoverTempStorage {
            funds: vec![Coin::new(1_000_000, "osmo")],
            recovery_addr,
        }),
        expected_messages: vec![],
    };

    test_reply(params).unwrap();
}
