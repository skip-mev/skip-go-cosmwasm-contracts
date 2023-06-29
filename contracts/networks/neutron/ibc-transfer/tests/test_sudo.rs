use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    BankMsg, Binary, Coin,
    ReplyOn::Never,
    StdError, SubMsg,
};
use neutron_sdk::sudo::msg::{RequestPacket, TransferSudoMsg};
use skip::ibc::NeutronInProgressIbcTransfer as InProgressIBCTransfer;
use skip_swap_neutron_ibc_transfer::{
    error::{ContractError, ContractResult},
    state::ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER,
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - Sudo Response - Happy Path - Send Timeout Fee
    - Sudo Timeout - Send Ibc Coin And Ack Fee Same Denom
    - Sudo Timeout - Send Ibc Coin And Ack Fee Different Denom
    - Sudo Error - Send Ibc Coin And Timeout Fee Same Denom
    - Sudo Error - Send Ibc Coin And Timeout Fee Different Denom

Expect Error
    - No In Progress Ibc Transfer Mapped To Sudo Ack ID - Expect Error
    - No channel id in TransferSudoMsg - Expect Error
    - No sequence in TransferSudoMsg - Expect Error

 */

// Define test parameters
struct Params {
    channel_id: String,
    sequence_id: u64,
    sudo_msg: TransferSudoMsg,
    stored_in_progress_ibc_transfer: Option<InProgressIBCTransfer>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test sudo
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Response {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            data: Binary::from(b""),
        },
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            ack_fee: vec![Coin::new(10, "osmo")],
            timeout_fee: vec![Coin::new(20, "osmo")],
        }),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(20, "osmo")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Sudo Response - Happy Path - Send Timeout Fee")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Timeout {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
        },
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            ack_fee: vec![Coin::new(10, "osmo")],
            timeout_fee: vec![Coin::new(20, "osmo")],
        }),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(110, "osmo")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Sudo Timeout - Send Ibc Coin And Ack Fee Same Denom")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Timeout {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
        },
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            ack_fee: vec![Coin::new(10, "ntrn")],
            timeout_fee: vec![Coin::new(10, "osmo")],
        }),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![ Coin::new(10, "ntrn"), Coin::new(100, "osmo")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Sudo Timeout - Send Ibc Coin And Ack Fee Different Denom")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Error {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            details: "".to_string(),
        },
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            ack_fee: vec![Coin::new(20, "osmo")],
            timeout_fee: vec![Coin::new(10, "osmo")],
        }),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(110, "osmo")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Sudo Error - Send Ibc Coin And Timeout Fee Same Denom")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Error {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            details: "".to_string(),
        },
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            ack_fee: vec![Coin::new(20, "osmo")],
            timeout_fee: vec![Coin::new(10, "ntrn")],
        }),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![ Coin::new(10, "ntrn"), Coin::new(100, "osmo")],
                }.into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "Sudo Error - Send Ibc Coin And Timeout Fee Different Denom")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Error {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            details: "".to_string(),
        },
        stored_in_progress_ibc_transfer: None,
        expected_messages: vec![],
        expected_error: Some(ContractError::Std(StdError::NotFound {
            kind: "skip::ibc::NeutronInProgressIbcTransfer".to_string(),
        })),
    };
    "No In Progress Ibc Transfer Mapped To Sudo Ack ID - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Error {
            request: RequestPacket {
                sequence: Some(1),
                source_port: None,
                source_channel: None,
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            details: "".to_string(),
        },
        stored_in_progress_ibc_transfer: None,
        expected_messages: vec![],
        expected_error: Some(ContractError::ChannelIDNotFound),
    };
    "No channel id in TransferSudoMsg - Expect Error")]
#[test_case(
    Params {
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: TransferSudoMsg::Error {
            request: RequestPacket {
                sequence: None,
                source_port: None,
                source_channel: Some("channel_id".to_string()),
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            details: "".to_string(),
        },
        stored_in_progress_ibc_transfer: None,
        expected_messages: vec![],
        expected_error: Some(ContractError::SequenceNotFound),
    };
    "No sequence in TransferSudoMsg - Expect Error")]
fn test_sudo(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let env = mock_env();

    // Store the in progress ibc transfer to state if it exists
    if let Some(in_progress_ibc_transfer) = params.stored_in_progress_ibc_transfer.clone() {
        ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.save(
            deps.as_mut().storage,
            (&params.channel_id, params.sequence_id),
            &in_progress_ibc_transfer,
        )?;
    }

    // Call sudo with the given test parameters
    let res = skip_swap_neutron_ibc_transfer::contract::sudo(deps.as_mut(), env, params.sudo_msg);

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Verify the in progress ibc transfer was removed from storage
            match ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER
                .load(&deps.storage, (&params.channel_id, params.sequence_id))
            {
                Ok(in_progress_ibc_transfer) => {
                    panic!(
                        "expected in progress ibc transfer to be removed: {:?}",
                        in_progress_ibc_transfer
                    )
                }
                Err(err) => assert_eq!(
                    err,
                    StdError::NotFound {
                        kind: "skip::ibc::NeutronInProgressIbcTransfer".to_string()
                    }
                ),
            };

            // Verify the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages);
        }
        Err(err) => {
            println!("Here");
            // Assert the test expected an error
            assert!(
                params.expected_error.is_some(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert_eq!(err, params.expected_error.unwrap());

            if params.stored_in_progress_ibc_transfer.is_some() {
                // Verify the ack id to in progress ibc transfer map entry is still stored
                assert_eq!(
                    ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER
                        .load(&deps.storage, (&params.channel_id, params.sequence_id))?,
                    params.stored_in_progress_ibc_transfer.unwrap()
                );
            }
        }
    }

    Ok(())
}
