use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Reply, StdError, SubMsgResponse, SubMsgResult,
};
use neutron_proto::neutron::transfer::MsgTransferResponse;
use prost::Message;
use skip_swap_neutron_ibc_transfer::{
    error::{ContractError, ContractResult},
    state::{ACK_ID_TO_RECOVER_ADDRESS, IN_PROGRESS_RECOVER_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - Happy Path (tests the in progress ibc transfer is removed from storage and the ack id to in progress ibc transfer map entry is correct)

Expect Error
    - Missing Sub Msg Response Data
    - Invalid Sub Msg Response Data To Convert To MsgTransferResponse
    - No In Progress Ibc Transfer To Load
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
    store_ack_id_to_in_progress_recover_address: bool,
    expected_error: Option<ContractError>,
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
                data: Some(MsgTransferResponse {sequence_id: 5, channel: "channel_id".to_string() }.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        store_ack_id_to_in_progress_recover_address: false,
        expected_error: None,
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
        store_ack_id_to_in_progress_recover_address: false,
        expected_error: Some(ContractError::MissingResponseData),
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
        store_ack_id_to_in_progress_recover_address: false,
        expected_error: Some(ContractError::Decode(prost::DecodeError::new("buffer underflow".to_string()))),
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
                data: Some(MsgTransferResponse {sequence_id: 1, channel: "channel_id".to_string() }.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: None,
        store_ack_id_to_in_progress_recover_address: false,
        expected_error: Some(ContractError::Std(StdError::NotFound { kind: "alloc::string::String".to_string() })),
    };
    "No In Progress Ibc Transfer To Load - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 5,
        reply: Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(MsgTransferResponse {sequence_id: 5, channel: "channel_id".to_string() }.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_recover_address: Some("recover_address".to_string()),
        store_ack_id_to_in_progress_recover_address: true,
        expected_error: Some(ContractError::AckIDAlreadyExists { channel_id: "channel_id".to_string(), sequence_id: 5 }),
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
        store_ack_id_to_in_progress_recover_address: false,
        expected_error: None,
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
        expected_error: None,
        store_ack_id_to_in_progress_recover_address: false,
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

    // If the test expects the ack id to in progress recover address map entry to be stored,
    // store it to state
    if params.store_ack_id_to_in_progress_recover_address {
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
    let res = skip_swap_neutron_ibc_transfer::contract::reply(deps.as_mut(), env, params.reply);

    // Assert the behavior is correct
    match res {
        Ok(_) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Verify the in progress recover address was removed from storage
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

            // Verify the stored ack id to in progress recover address map entry is correct
            assert_eq!(
                ACK_ID_TO_RECOVER_ADDRESS
                    .load(&deps.storage, (&params.channel_id, params.sequence_id))?,
                params.pre_reply_in_progress_recover_address.unwrap()
            );
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
