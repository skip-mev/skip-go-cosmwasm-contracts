use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env},
    Addr, BankMsg, Binary, Coin,
    ReplyOn::Never,
    StdError, SubMsg,
};
use neutron_sdk::sudo::msg::{RequestPacket, TransferSudoMsg};
use skip_go_ibc_adapter_neutron_transfer::{
    error::{ContractError, ContractResult},
    state::ACK_ID_TO_RECOVER_ADDRESS,
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
    - No In Progress Recover Address Mapped To Sudo Ack ID - Expect Error
    - No channel id in TransferSudoMsg - Expect Error
    - No sequence in TransferSudoMsg - Expect Error
    - No Contract Balance To Refund - Expect Error

 */

// Define test parameters
struct Params {
    contract_balance: Vec<Coin>,
    channel_id: String,
    sequence_id: u64,
    sudo_msg: TransferSudoMsg,
    stored_in_progress_recover_address: Option<String>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test sudo
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: Some("recover_address".to_string()),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(100, "uosmo")],
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
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: Some("recover_address".to_string()),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(100, "uosmo")],
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
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: Some("recover_address".to_string()),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(100, "uosmo")],
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
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: Some("recover_address".to_string()),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(100, "uosmo")],
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
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: Some("recover_address".to_string()),
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: BankMsg::Send {
                    to_address: "recover_address".to_string(),
                    amount: vec![Coin::new(100, "uosmo")],
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
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: None,
        expected_messages: vec![],
        expected_error: Some(ContractError::Std(StdError::NotFound {
            kind: "type: alloc::string::String; key: [00, 19, 61, 63, 6B, 5F, 69, 64, 5F, 74, 6F, 5F, 72, 65, 63, 6F, 76, 65, 72, 5F, 61, 64, 64, 72, 65, 73, 73, 00, 0A, 63, 68, 61, 6E, 6E, 65, 6C, 5F, 69, 64, 00, 00, 00, 00, 00, 00, 00, 01]".to_string(),
        })),
    };
    "No In Progress Ibc Transfer Mapped To Sudo Ack ID - Expect Error")]
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: None,
        expected_messages: vec![],
        expected_error: Some(ContractError::ChannelIDNotFound),
    };
    "No channel id in TransferSudoMsg - Expect Error")]
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
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
        stored_in_progress_recover_address: None,
        expected_messages: vec![],
        expected_error: Some(ContractError::SequenceNotFound),
    };
    "No sequence in TransferSudoMsg - Expect Error")]
#[test_case(
    Params {
        contract_balance: vec![],
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
        stored_in_progress_recover_address: Some("recover_address".to_string()),
        expected_messages: vec![],
        expected_error: Some(ContractError::NoFundsToRefund),
    };
    "No Contract Balance To Refund - Expect Error")]
fn test_sudo(params: Params) -> ContractResult<()> {
    // Convert params contract balance to a slice
    let contract_balance: &[Coin] = &params.contract_balance;

    // Create mock dependencies
    let mut deps = mock_dependencies_with_balances(&[("ibc_transfer_adapter", contract_balance)]);

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("ibc_transfer_adapter");

    // Store the in progress recover address to state if it exists
    if let Some(in_progress_recover_address) = params.stored_in_progress_recover_address.clone() {
        ACK_ID_TO_RECOVER_ADDRESS.save(
            deps.as_mut().storage,
            (&params.channel_id, params.sequence_id),
            &in_progress_recover_address,
        )?;
    }

    // Call sudo with the given test parameters
    let res =
        skip_go_ibc_adapter_neutron_transfer::contract::sudo(deps.as_mut(), env, params.sudo_msg);

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Verify the in progress recover address was removed from storage
            match ACK_ID_TO_RECOVER_ADDRESS
                .load(&deps.storage, (&params.channel_id, params.sequence_id))
            {
                Ok(in_progress_recover_address) => {
                    panic!(
                        "expected in progress recover address to be removed: {:?}",
                        in_progress_recover_address
                    )
                }
                Err(err) => assert!(
                    matches!(err, StdError::NotFound { .. }),
                    "unexpected error: {:?}",
                    err
                ),
            };

            // Verify the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages);
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
