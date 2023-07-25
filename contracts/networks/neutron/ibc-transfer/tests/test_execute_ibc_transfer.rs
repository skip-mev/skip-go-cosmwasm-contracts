use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as CosmosSdkCoin;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Coin,
    ReplyOn::Success,
    SubMsg, Uint128,
};
use neutron_proto::neutron::{feerefunder::Fee as NeutronFee, transfer::MsgTransfer};
use skip::ibc::{ExecuteMsg, IbcFee, IbcInfo};
use skip_swap_neutron_ibc_transfer::{
    error::ContractResult,
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_RECOVER_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Happy Path (tests the message emitted is expected and the in progress ibc transfer is saved correctly)

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
 */

// Define test parameters
struct Params {
    caller: String,
    ibc_adapter_contract_address: Addr,
    coin: Coin,
    ibc_info: IbcInfo,
    timeout_timestamp: u64,
    expected_messages: Vec<SubMsg>,
    expected_error_string: String,
}

// Test execute_ibc_transfer
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: IbcFee {
                recv_fee: vec![],
                ack_fee: vec![Coin {
                    denom: "ntrn".to_string(),
                    amount: Uint128::new(10),
                }],
                timeout_fee: vec![],
            },
            memo: "memo".to_string(),
            recover_address: "recover_address".to_string(),
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            msg: MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: "source_channel".to_string(),
                token: Some(CosmosSdkCoin {
                    denom: "osmo".to_string(),
                    amount: "100".to_string(),
                }),
                sender: "ibc_transfer".to_string(),
                receiver: "receiver".to_string(),
                timeout_height: None,
                timeout_timestamp: 100,
                memo: "memo".to_string(),
                fee: Some(NeutronFee {
                    recv_fee: vec![],
                    ack_fee: vec![CosmosSdkCoin {
                        denom: "ntrn".to_string(),
                        amount: "10".to_string(),
                    }],
                    timeout_fee: vec![],
                }),
            }
            .into(),
            gas_limit: None,
            reply_on: Success,
        }],
        expected_error_string: "".to_string(),
    };
    "Happy Path")]
#[test_case(
    Params {
        caller: "random".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: IbcFee {
                recv_fee: vec![],
                ack_fee: vec![Coin {
                    denom: "ntrn".to_string(),
                    amount: Uint128::new(10),
                }],
                timeout_fee: vec![],
            },
            memo: "memo".to_string(),
            recover_address: "recover_address".to_string(),
        },
        timeout_timestamp: 100,
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
    };
    "Unauthorized Caller - Expect Error")]
fn test_execute_ibc_transfer(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let mut env = mock_env();
    env.contract.address = params.ibc_adapter_contract_address.clone();

    // Create mock info
    let info = mock_info(&params.caller, &[]);

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;

    // Call execute_ibc_transfer with the given test parameters
    let res = skip_swap_neutron_ibc_transfer::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::IbcTransfer {
            info: params.ibc_info.clone(),
            coin: params.coin.clone(),
            timeout_timestamp: params.timeout_timestamp,
        },
    );

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error_string.is_empty(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error_string
            );

            // Assert the messages in the response are correct
            assert_eq!(res.messages, params.expected_messages);

            // Load the in progress ibc transfer from state and verify it is correct
            let stored_in_progress_recover_address =
                IN_PROGRESS_RECOVER_ADDRESS.load(&deps.storage)?;

            // Assert the in progress ibc transfer is correct
            assert_eq!(
                stored_in_progress_recover_address,
                params.ibc_info.recover_address
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
