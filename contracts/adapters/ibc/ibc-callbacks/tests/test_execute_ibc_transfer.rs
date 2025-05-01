#![allow(deprecated)]

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Binary, Coin,
    ReplyOn::Success,
    SubMsg,
};
use ibc_proto::cosmos::base::v1beta1::Coin as IbcCoin;
use ibc_proto::ibc::applications::transfer::v1::MsgTransfer;
use prost::Message;
use skip2::ibc::{ExecuteMsg, IbcFee, IbcInfo};
use skip_go_ibc_adapter_ibc_callbacks::{
    error::ContractResult,
    state::{ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_CHANNEL_ID, IN_PROGRESS_RECOVER_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Response (Output Message Is Correct, In Progress Ibc Transfer Is Saved, No Error)
    - Empty String Memo
    - Override Already Set Source Ibc Callback Memo
    - Add Ibc Source Callback Key/Value Pair To Other Key/Value In Memo
    - Valid EVM Address

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - Non Empty String, Invalid Json Memo
    - Non Empty IBC Fees, IBC Fees Not Supported
    - Invalid EVM Address When Solidiy Encoding Is Provided

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
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: "".to_string(),
            recover_address: "recover_address".to_string(),
            encoding: None,
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            payload: Binary::default(),
            msg: cosmwasm_std::CosmosMsg::Stargate {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: MsgTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: "source_channel".to_string(),
                    token: Some(IbcCoin {
                        denom: "osmo".to_string(),
                        amount: "100".to_string(),
                    }),
                    sender: "ibc_transfer".to_string(),
                    receiver: "receiver".to_string(),
                    timeout_height: None,
                    timeout_timestamp: 100,
                    memo: r#"{"src_callback":{"address":"ibc_transfer"}}"#.to_string(),
                    encoding: "".to_string(),
                }
                .encode_to_vec().into(),
            },
            gas_limit: None,
            reply_on: Success,
        }
        ],
        expected_error_string: "".to_string(),
    };
    "Empty String Memo")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: r#"{"src_callback":{"address":"random"}}"#.to_string(),
            recover_address: "recover_address".to_string(),
            encoding: None,
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            payload: Binary::default(),
            msg: cosmwasm_std::CosmosMsg::Stargate {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: MsgTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: "source_channel".to_string(),
                    token: Some(IbcCoin {
                        denom: "osmo".to_string(),
                        amount: "100".to_string(),
                    }),
                    sender: "ibc_transfer".to_string(),
                    receiver: "receiver".to_string(),
                    timeout_height: None,
                    timeout_timestamp: 100,
                    memo: r#"{"src_callback":{"address":"ibc_transfer"}}"#.to_string(),
                    encoding: "".to_string(),
                }
                .encode_to_vec().into(),
            },
            gas_limit: None,
            reply_on: Success,
        }
        ],
        expected_error_string: "".to_string(),
    };
    "Override Already Set Ibc Source Callback Memo")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: r#"{"pfm":"example_value","wasm":"example_contract"}"#.to_string(),
            recover_address: "recover_address".to_string(),
            encoding: None,
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            payload: Binary::default(),
            msg: cosmwasm_std::CosmosMsg::Stargate {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: MsgTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: "source_channel".to_string(),
                    token: Some(IbcCoin {
                        denom: "osmo".to_string(),
                        amount: "100".to_string(),
                    }),
                    sender: "ibc_transfer".to_string(),
                    receiver: "receiver".to_string(),
                    timeout_height: None,
                    timeout_timestamp: 100,
                    memo: r#"{"pfm":"example_value","src_callback":{"address":"ibc_transfer"},"wasm":"example_contract"}"#.to_string(),
                    encoding: "".to_string(),
                }
                .encode_to_vec().into(),
            },
            gas_limit: None,
            reply_on: Success,
        }
        ],
        expected_error_string: "".to_string(),
    };
    "Add Ibc Source Callback Key/Value Pair To Other Key/Value In Memo")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "0x24a9267cE9e0a8F4467B584FDDa12baf1Df772B5".to_string(),
            fee: None,
            memo: "".to_string(),
            recover_address: "recover_address".to_string(),
            encoding: Some("application/x-solidity-abi".to_string()),
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            payload: Binary::default(),
            msg: cosmwasm_std::CosmosMsg::Stargate {
                type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                value: MsgTransfer {
                    source_port: "transfer".to_string(),
                    source_channel: "source_channel".to_string(),
                    token: Some(IbcCoin {
                        denom: "osmo".to_string(),
                        amount: "100".to_string(),
                    }),
                    sender: "ibc_transfer".to_string(),
                    receiver: "0x24a9267cE9e0a8F4467B584FDDa12baf1Df772B5".to_string(),
                    timeout_height: None,
                    timeout_timestamp: 100,
                    memo: r#"{"src_callback":{"address":"ibc_transfer"}}"#.to_string(),
                    encoding: "application/x-solidity-abi".to_string(),
                }
                .encode_to_vec().into(),
            },
            gas_limit: None,
            reply_on: Success,
        }
        ],
        expected_error_string: "".to_string(),
    };
    "Valid EVM Address When Solidiy Encoding Is Provided")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: "{invalid}".to_string(),
            recover_address: "recover_address".to_string(),
            encoding: None,
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![],
        expected_error_string: "Generic error: Error parsing memo: Object key is not a string.".to_string(),
    };
    "Non Empty String, Invalid Json Memo - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: Some(IbcFee {
                recv_fee: vec![
                    Coin::new(100u128, "atom"),
                ],
                ack_fee: vec![],
                timeout_fee: vec![],
            }),
            memo: "{}".to_string(),
            recover_address: "recover_address".to_string(),
            encoding: None,
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![],
        expected_error_string: "IBC fees are not supported, vectors must be empty".to_string(),
    };
    "IBC Fees Not Supported - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: "{}".to_string(),
            recover_address: "recover_address".to_string(),
            encoding: None,
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
    };
    "Unauthorized Caller - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        coin: Coin::new(100u128, "osmo"),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "cosmos1zhqrfu9w3sugwykef3rq8t0vlxkz72vw9pzsvv".to_string(),
            fee: None,
            memo: "{}".to_string(),
            recover_address: "recover_address".to_string(),
            encoding: Some("application/x-solidity-abi".to_string()),
            eureka_fee: None,
        },
        timeout_timestamp: 100,
        expected_messages: vec![],
        expected_error_string: "EVM Address provided is invalid".to_string(),
    };
        "Invalid EVM Address When Solidiy Encoding Is Provided - Expect Error")]
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
    let res = skip_go_ibc_adapter_ibc_callbacks::contract::execute(
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

            // Load the in progress recover address from state and verify it is correct
            let stored_in_progress_recover_address =
                IN_PROGRESS_RECOVER_ADDRESS.load(&deps.storage)?;

            // Assert the in progress recover address is correct
            assert_eq!(
                stored_in_progress_recover_address,
                params.ibc_info.recover_address
            );

            // Load the in progress channel id from state and verify it is correct
            let stored_in_progress_channel_id = IN_PROGRESS_CHANNEL_ID.load(&deps.storage)?;

            // Assert the in progress channel id is correct
            assert_eq!(
                stored_in_progress_channel_id,
                params.ibc_info.source_channel
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
