use cosmwasm_std::{
    testing::{mock_dependencies_with_balances, mock_env},
    Addr, BankMsg, Coin,
    ReplyOn::Never,
    StdError, SubMsg,
};
use skip::{
    ibc::{IbcLifecycleComplete, OsmosisInProgressIbcTransfer as InProgressIBCTransfer},
    sudo::OsmosisSudoMsg as SudoMsg,
};
use skip_swap_osmosis_ibc_transfer::{
    error::ContractResult, state::ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER,
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - Sudo Response - Happy Path Response
    - Sudo Timeout - Send Failed Ibc Coin To Recover Address
    - Sudo Error - Send Failed Ibc Coin To Recover Address

Expect Error
    - No In Progress Ibc Transfer Mapped To Sudo Ack ID - Expect Error
    - No Contract Balance To Refund - Expect Error

 */

// Define test parameters
struct Params {
    contract_balance: Vec<Coin>,
    channel_id: String,
    sequence_id: u64,
    sudo_msg: SudoMsg,
    stored_in_progress_ibc_transfer: Option<InProgressIBCTransfer>,
    expected_messages: Vec<SubMsg>,
    expected_error_string: String,
}

// Test sudo
#[test_case(
    Params {
        contract_balance: vec![],
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcAck{
            channel: "channel_id".to_string(),
            sequence: 1,
            ack: "".to_string(),
            success: true,
        }),
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
        expected_messages: vec![],
        expected_error_string: "".to_string(),
    };
    "Sudo Response - Happy Path")]
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcTimeout{
            channel: "channel_id".to_string(),
            sequence: 1,
        }),
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
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
        expected_error_string: "".to_string(),
    };
    "Sudo Timeout - Send Failed Ibc Coin")]
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcAck{
            channel: "channel_id".to_string(),
            sequence: 1,
            ack: "".to_string(),
            success: false,
        }),
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
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
        expected_error_string: "".to_string(),
    };
    "Sudo Error - Send Failed Ibc Coin To Recover Address")]
#[test_case(
    Params {
        contract_balance: vec![Coin::new(100, "uosmo")],
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcAck{
            channel: "channel_id".to_string(),
            sequence: 1,
            ack: "".to_string(),
            success: false,
        }),
        stored_in_progress_ibc_transfer: None,
        expected_messages: vec![],
        expected_error_string: "skip::ibc::OsmosisInProgressIbcTransfer not found".to_string(),
    };
    "No In Progress Ibc Transfer Mapped To Sudo Ack ID - Expect Error")]
#[test_case(
    Params {
        contract_balance: vec![],
        channel_id: "channel_id".to_string(),
        sequence_id: 1,
        sudo_msg: SudoMsg::IbcLifecycleComplete(IbcLifecycleComplete::IbcAck{
            channel: "channel_id".to_string(),
            sequence: 1,
            ack: "".to_string(),
            success: false,
        }),
        stored_in_progress_ibc_transfer: Some(InProgressIBCTransfer {
            recover_address: "recover_address".to_string(),
            coin: Coin::new(100, "osmo"),
            channel_id: "channel_id".to_string(),
        }),
        expected_messages: vec![],
        expected_error_string: "Failed to receive ibc funds to refund the user".to_string(),
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

    // Store the in progress ibc transfer to state if it exists
    if let Some(in_progress_ibc_transfer) = params.stored_in_progress_ibc_transfer.clone() {
        ACK_ID_TO_IN_PROGRESS_IBC_TRANSFER.save(
            deps.as_mut().storage,
            (&params.channel_id, params.sequence_id),
            &in_progress_ibc_transfer,
        )?;
    }

    // Call sudo with the given test parameters
    let res = skip_swap_osmosis_ibc_transfer::contract::sudo(deps.as_mut(), env, params.sudo_msg);

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error_string.is_empty(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error_string
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
                        kind: "skip::ibc::OsmosisInProgressIbcTransfer".to_string()
                    }
                ),
            };

            // Verify the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages);
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
