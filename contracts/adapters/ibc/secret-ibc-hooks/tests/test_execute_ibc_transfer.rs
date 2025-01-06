use cosmwasm_std::{
    to_binary,
    WasmMsg,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Coin, ContractInfo,
    ReplyOn::Success,
    SubMsg,
};
use ibc_proto::cosmos::base::v1beta1::Coin as IbcCoin;
use ibc_proto::ibc::applications::transfer::v1::MsgTransfer;
use prost::Message;
use secret_skip::{
    asset::Asset,
    ibc::{ExecuteMsg, IbcFee, IbcInfo, Snip20HookMsg,
    Ics20TransferMsg},
    snip20::{self, Snip20ReceiveMsg},
    cw20::Cw20Coin,
};
use skip_go_secret_ibc_adapter_ibc_hooks::{
    error::ContractResult,
    state::{
        ENTRY_POINT_CONTRACT, ICS20_CONTRACT, IN_PROGRESS_CHANNEL_ID,
        IN_PROGRESS_RECOVER_ADDRESS, REGISTERED_TOKENS, VIEWING_KEY,
    },
};
use test_case::test_case;

/*
Test Cases:

Expect Response (Output Message Is Correct, In Progress Ibc Transfer Is Saved, No Error)
    - Empty String Memo
    - Override Already Set Ibc Callback Memo
    - Add Ibc Callback Key/Value Pair To Other Key/Value In Memo

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - Non Empty String, Invalid Json Memo
    - Non Empty IBC Fees, IBC Fees Not Supported

 */

// Define test parameters
struct Params {
    caller: String,
    ibc_adapter_contract_address: Addr,
    sent_asset: Asset,
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
        sent_asset: Asset::Cw20(Cw20Coin { 
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: "".to_string(),
            recover_address: "recover_address".to_string(),
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            msg: WasmMsg::Execute {
                contract_addr: "secret123".to_string(),
                code_hash: "code_hash".to_string(),
                msg: to_binary(&snip20::ExecuteMsg::Send {
                    amount: 100u128.into(),
                    recipient: "ics20".to_string(),
                    recipient_code_hash: Some("code_hash".to_string()),
                    memo: Some(r#"{"ibc_callback":"ibc_transfer"}"#.to_string()),
                    padding: None,
                    msg: Some(to_binary(&Ics20TransferMsg {
                        channel: "source_channel".to_string(),
                        remote_address: "receiver".to_string(),
                        timeout: Some(100),
                    })?),
                })?,
                funds: vec![],
            }.into(),
            gas_limit: None,
            reply_on: Success,
        }],
        expected_error_string: "".to_string(),
    };
    "Empty String Memo")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        sent_asset: Asset::Cw20(Cw20Coin { 
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: r#"{"ibc_callback":"random_address"}"#.to_string(),
            recover_address: "recover_address".to_string(),
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            msg: WasmMsg::Execute {
                contract_addr: "secret123".to_string(),
                code_hash: "code_hash".to_string(),
                msg: to_binary(&snip20::ExecuteMsg::Send {
                    amount: 100u128.into(),
                    recipient: "ics20".to_string(),
                    recipient_code_hash: Some("code_hash".to_string()),
                    memo: Some(r#"{"ibc_callback":"ibc_transfer"}"#.to_string()),
                    padding: None,
                    msg: Some(to_binary(&Ics20TransferMsg {
                        channel: "source_channel".to_string(),
                        remote_address: "receiver".to_string(),
                        timeout: Some(100),
                    })?),
                })?,
                funds: vec![],
            }.into(),
            gas_limit: None,
            reply_on: Success,
        }
        ],
        expected_error_string: "".to_string(),
    };
    "Override Already Set Ibc Callback Memo")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        sent_asset: Asset::Cw20(Cw20Coin { 
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: r#"{"pfm":"example_value","wasm":"example_contract"}"#.to_string(),
            recover_address: "recover_address".to_string(),
        },
        timeout_timestamp: 100,
        expected_messages: vec![SubMsg {
            id: 1,
            msg: WasmMsg::Execute {
                contract_addr: "secret123".to_string(),
                code_hash: "code_hash".to_string(),
                msg: to_binary(&snip20::ExecuteMsg::Send {
                    amount: 100u128.into(),
                    recipient: "ics20".to_string(),
                    recipient_code_hash: Some("code_hash".to_string()),
                    memo: Some(r#"{"ibc_callback":"ibc_transfer","pfm":"example_value","wasm":"example_contract"}"#.to_string()),
                    padding: None,
                    msg: Some(to_binary(&Ics20TransferMsg {
                        channel: "source_channel".to_string(),
                        remote_address: "receiver".to_string(),
                        timeout: Some(100),
                    })?),
                })?,
                funds: vec![],
            }.into(),
            gas_limit: None,
            reply_on: Success,
        }],
        expected_error_string: "".to_string(),
    };
    "Add Ibc Callback Key/Value Pair To Other Key/Value In Memo")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        sent_asset: Asset::Cw20(Cw20Coin { 
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: "{invalid}".to_string(),
            recover_address: "recover_address".to_string(),
        },
        timeout_timestamp: 100,
        expected_messages: vec![],
        expected_error_string: "Object key is not a string.".to_string(),
    };
    "Non Empty String, Invalid Json Memo - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        sent_asset: Asset::Cw20(Cw20Coin { 
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: Some(IbcFee {
                recv_fee: vec![
                    Coin::new(100, "atom"),
                ],
                ack_fee: vec![],
                timeout_fee: vec![],
            }),
            memo: "{}".to_string(),
            recover_address: "recover_address".to_string(),
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
        sent_asset: Asset::Cw20(Cw20Coin { 
            address: "secret123".to_string(),
            amount: 100u128.into(),
        }),
        ibc_info: IbcInfo {
            source_channel: "source_channel".to_string(),
            receiver: "receiver".to_string(),
            fee: None,
            memo: "{}".to_string(),
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
    env.contract.code_hash = "code_hash".to_string();


    // Store the entry point contract address
    ENTRY_POINT_CONTRACT.save(deps.as_mut().storage, &ContractInfo {
        address: Addr::unchecked("entry_point"),
        code_hash: "code_hash".to_string(),
    })?;

    ICS20_CONTRACT.save(
        deps.as_mut().storage,
        &ContractInfo {
            address: Addr::unchecked("ics20"),
            code_hash: "code_hash".to_string(),
        },
    )?;
    REGISTERED_TOKENS.save(
        deps.as_mut().storage,
        Addr::unchecked("secret123"),
        &ContractInfo {
            address: Addr::unchecked("secret123"),
            code_hash: "code_hash".to_string(),
        },
    )?;
    VIEWING_KEY.save(deps.as_mut().storage, &"viewing_key".to_string())?;

    // Call execute_ibc_transfer with the given test parameters
    let res = skip_go_secret_ibc_adapter_ibc_hooks::contract::execute(
        deps.as_mut(),
        env,
        mock_info(&"secret123", &[]),
        ExecuteMsg::Receive(Snip20ReceiveMsg {
            sender: Addr::unchecked(params.caller.clone()),
            amount: params.sent_asset.amount(),
            from: Addr::unchecked(params.caller),
            memo: None,
            msg: Some(
                to_binary(&Snip20HookMsg::IbcTransfer {
                    info: params.ibc_info.clone(),
                    timeout_timestamp: params.timeout_timestamp,
                })
                .unwrap(),
            ),
        }),
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
