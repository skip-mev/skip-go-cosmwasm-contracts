use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Coin, Reply, StdError, SubMsgResponse, SubMsgResult,
};
use ibc_proto::ibc::applications::transfer::v1::MsgTransferResponse;
use prost::Message;
use skip::ibc::OsmosisInProgressIbcTransfer as InProgressIBCTransfer;
use skip_swap_osmosis_ibc_transfer::{
    error::ContractResult,
    state::{ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER, IN_PROGRESS_IBC_TRANSFER},
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
    - SubMsg Incorrect Reply ID

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
    pre_reply_in_progress_ibc_transfer: Option<InProgressIBCTransfer>,
    store_ack_id_to_in_progress_ibc_transfer: bool,
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
        pre_reply_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
        store_ack_id_to_in_progress_ibc_transfer: false,
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
        pre_reply_in_progress_ibc_transfer: None,
        store_ack_id_to_in_progress_ibc_transfer: false,
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
        pre_reply_in_progress_ibc_transfer: None,
        store_ack_id_to_in_progress_ibc_transfer: false,
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
        pre_reply_in_progress_ibc_transfer: None,
        store_ack_id_to_in_progress_ibc_transfer: false,
        expected_error_string: "skip::ibc::OsmosisInProgressIbcTransfer not found".to_string(),
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
                data: Some(MsgTransferResponse {sequence: 5}.encode_to_vec().as_slice().into()),
            }),
        },
        pre_reply_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
        store_ack_id_to_in_progress_ibc_transfer: true,
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
        pre_reply_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
        store_ack_id_to_in_progress_ibc_transfer: false,
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
        pre_reply_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
        expected_error_string: "".to_string(),
        store_ack_id_to_in_progress_ibc_transfer: false,
    } => panics "internal error: entered unreachable code";
    "SubMsgResult Error - Expect Panic")]
fn test_reply(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let env = mock_env();

    // Store the in progress ibc transfer to state if it exists
    if let Some(in_progress_ibc_transfer) = params.pre_reply_in_progress_ibc_transfer.clone() {
        IN_PROGRESS_IBC_TRANSFER.save(deps.as_mut().storage, &in_progress_ibc_transfer)?;
    }

    // If the test expects the ack id to in progress ibc transfer map entry to be stored,
    // store it to state
    if params.store_ack_id_to_in_progress_ibc_transfer {
        ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.save(
            deps.as_mut().storage,
            (&params.channel_id, params.sequence_id),
            &params.pre_reply_in_progress_ibc_transfer.clone().unwrap(),
        )?;
    }

    // Call reply with the given test parameters
    let res = skip_swap_osmosis_ibc_transfer::contract::reply(deps.as_mut(), env, params.reply);

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
            match IN_PROGRESS_IBC_TRANSFER.load(&deps.storage) {
                Ok(in_progress_ibc_transfer) => {
                    panic!(
                        "expected in progress ibc transfer to be removed: {:?}",
                        in_progress_ibc_transfer
                    )
                }
                Err(err) => assert_eq!(
                    err,
                    StdError::NotFound {
                        kind: "skip::ibc::OsmosisInProgressIbcTransfer".to_string()
                    }
                ),
            };

            // Verify the stored ack id to in progress ibc transfer map entry is correct
            assert_eq!(
                ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER
                    .load(&deps.storage, (&params.channel_id, params.sequence_id))?,
                params.pre_reply_in_progress_ibc_transfer.unwrap()
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
