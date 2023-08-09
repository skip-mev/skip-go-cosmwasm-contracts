use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Reply, StdError, SubMsgResponse, SubMsgResult,
};
use ibc_proto::ibc::applications::transfer::v1::MsgTransferResponse;
use prost::Message;
use skip_api_ibc_adapter_ibc_hooks::{
    error::ContractResult,
    state::{ACK_ID_TO_RECOVER_ADDRESS, IN_PROGRESS_CHANNEL_ID, IN_PROGRESS_RECOVER_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - Happy Path (tests the in progress ibc transfer is removed from storage and the ack id to in progress ibc transfer map entry is correct)

Expect Error
    - Missing Sub Msg Response Data
    - Invalid Sub Msg Response Data To Convert To MsgTransferResponse
    - No In Progress Recover Address To Load
    - No In Progress Channel ID To Load
    - Ack ID Already Exists

Expect Panic
    - SubMsgResult Error
        - Should panic because the sub msg is set to reply only on success, so should never happen
          unless the wasm module worked unexpectedly
    - SubMsg Incorrect Reply ID
        - Should panic because the reply id is set to a constant, so should never happen unless
          the wasm module worked unexpectedly
 */

// Define test parameters
struct Params {
    channel_id: String,
    sequence_id: u64,
    reply: Reply,
    pre_reply_in_progress_recover_address: Option<String>,
    pre_reply_in_progress_channel_id: Option<String>,
    store_ack_id_to_recover_address: bool,
    expected_error_string: String,
}

// Test reply
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 5,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(MsgTransferResponse {sequence: 5}.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        pre_reply_in_progress_channel_id: Some("channel_id".to_string()),
        store_ack_id_to_recover_address: false,
        expected_error_string: "".to_string(),
    };
    "Happy Path")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
        pre_reply_in_progress_recover_address: None,
        pre_reply_in_progress_channel_id: None,
        store_ack_id_to_recover_address: false,
        expected_error_string: "SubMsgResponse does not contain data".to_string(),
    };
    "Missing Sub Msg Response Data - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(b"invalid".into()),
            }),
        },
        pre_reply_in_progress_recover_address: None,
        pre_reply_in_progress_channel_id: None,
        store_ack_id_to_recover_address: false,
        expected_error_string: "failed to decode Protobuf message: buffer underflow".to_string(),
    };
    "Invalid Sub Msg Response Data To Convert To MsgTransferResponse - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(MsgTransferResponse {sequence: 5}.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: None,
        pre_reply_in_progress_channel_id: Some("channel_id".to_string()),
        store_ack_id_to_recover_address: false,
        expected_error_string: "alloc::string::String not found".to_string(),
    };
    "No In Progress Recover Address To Load - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(MsgTransferResponse {sequence: 5}.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        pre_reply_in_progress_channel_id: None,
        store_ack_id_to_recover_address: false,
        expected_error_string: "alloc::string::String not found".to_string(),
    };
    "No In Progress Channel ID To Load - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 5,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(MsgTransferResponse {sequence: 5}.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        pre_reply_in_progress_channel_id: Some("channel_id".to_string()),
        store_ack_id_to_recover_address: true,
        expected_error_string: "ACK ID already exists for channel ID channel_id and sequence ID 5".to_string(),
    };
    "Ack ID Already Exists - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        reply: Reply {
            id: 2,
            result: SubMsgResult::Err("".to_string()),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        pre_reply_in_progress_channel_id: Some("channel_id".to_string()),
        store_ack_id_to_recover_address: false,
        expected_error_string: "".to_string(),
    } => panics "internal error: entered unreachable code";
    "SubMsg Incorrect Reply ID - Expect Panic")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Err("".to_string()),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        pre_reply_in_progress_channel_id: Some("channel_id".to_string()),
        expected_error_string: "".to_string(),
        store_ack_id_to_recover_address: false,
    } => panics "internal error: entered unreachable code";
    "SubMsgResult Error - Expect Panic")]
fn test_reply(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let env = mock_env();

    // Store the in progress recover address to state if it exists
    if let Some(in_progress_recover_address) = params.pre_reply_in_progress_recover_address.clone()
    {
        IN_PROGRESS_RECOVER_ADDRESS.save(deps.as_mut().storage, &in_progress_recover_address)?;
    }

    // Store the in progress channel id to state if it exists
    if let Some(in_progress_channel_id) = params.pre_reply_in_progress_channel_id.clone() {
        IN_PROGRESS_CHANNEL_ID.save(deps.as_mut().storage, &in_progress_channel_id)?;
    }

    // If the test expects the ack id to in progress ibc transfer map entry to be stored,
    // store it to state
    if params.store_ack_id_to_recover_address {
        ACK_ID_TO_RECOVER_ADDRESS.save(
            deps.as_mut().storage,
            (&params.channel_id, params.sequence_id),
            &params
                .pre_reply_in_progress_recover_address
                .clone()
                .unwrap(),
        )?;
    }

    // Call reply with the given test parameters
    let res = skip_api_ibc_adapter_ibc_hooks::contract::reply(deps.as_mut(), env, params.reply);

    // Assert the behavior is correct
    match res {
        Ok(_) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error_string.is_empty(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error_string
            );

            // Verify the in progress ibc transfer was removed from storage
            match IN_PROGRESS_RECOVER_ADDRESS.load(&deps.storage) {
                Ok(in_progress_ibc_transfer) => {
                    panic!(
                        "expected in progress ibc transfer to be removed: {:?}",
                        in_progress_ibc_transfer
                    )
                }
                Err(err) => assert_eq!(
                    err,
                    StdError::NotFound {
                        kind: "alloc::string::String".to_string()
                    }
                ),
            };

            // Verify the stored ack id to in progress ibc transfer map entry is correct
            assert_eq!(
                ACK_ID_TO_RECOVER_ADDRESS
                    .load(&deps.storage, (&params.channel_id, params.sequence_id))?,
                params.pre_reply_in_progress_recover_address.unwrap()
            );
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
