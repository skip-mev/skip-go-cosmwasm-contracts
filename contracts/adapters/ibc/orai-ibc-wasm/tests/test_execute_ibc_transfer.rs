
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info}, to_json_binary, Addr, Coin, SubMsg, Uint128, WasmMsg
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw20_ics20_msg::msg::TransferBackMsg;
use skip::{asset::Asset, ibc_wasm::ExecuteMsg};
use skip_api_ibc_adapter_orai_ibc_wasm::{
    error::ContractResult, state::{ENTRY_POINT_CONTRACT_ADDRESS, IBC_WASM_CONTRACT_ADDRESS},
};
use test_case::test_case;

/*
Test Cases:

Expect Response
    - Happy Path (tests the message emitted is expected and the in progress ibc transfer is saved correctly)

Expect Error
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - No IBC Fees Provided (IBC fees are required for Osmosis)
 */

// Define test parameters
struct Params {
    caller: String,
    ibc_adapter_contract_address: Addr,
    asset: Asset,
    ibc_wasm_info: TransferBackMsg,
    expected_messages: Vec<SubMsg>,
    expected_error_string: String,
}

// Test execute_ibc_transfer
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        asset: Asset::Native(Coin::new(100, "osmo")),
        ibc_wasm_info: TransferBackMsg { local_channel_id: "source_channel".to_string(), remote_address: "orai123".to_string(), remote_denom: "oraib0x123".to_string(), timeout:None, memo: Some("oraib0x12".to_string()) },
       
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute { contract_addr: "ibc_wasm".to_string(), msg: to_json_binary(&skip::ibc_wasm::IbcWasmExecuteMsg::TransferToRemote(TransferBackMsg { local_channel_id: "source_channel".to_string(), remote_address: "orai123".to_string(), remote_denom: "oraib0x123".to_string(), timeout:None, memo: Some("oraib0x12".to_string()) }))?, funds: vec![Coin::new(100, "osmo")] } 
            .into(),
            gas_limit: None,
            reply_on: cosmwasm_std::ReplyOn::Never,
        }],
        expected_error_string: "".to_string(),
    };
    "Happy Path with native token")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        asset: Asset::Cw20(cw20::Cw20Coin {address: "usdt".to_string(), amount: Uint128::new(1000000)}),
        ibc_wasm_info: TransferBackMsg { local_channel_id: "source_channel".to_string(), remote_address: "orai123".to_string(), remote_denom: "oraib0x123".to_string(), timeout:None, memo: Some("oraib0x12".to_string()) },
       
        expected_messages: vec![SubMsg {
            id: 0,
            msg: WasmMsg::Execute { contract_addr: "usdt".to_string(), msg: to_json_binary(&Cw20ExecuteMsg::Send { contract: "ibc_wasm".to_string(), amount: Uint128::new(1000000), msg:  to_json_binary(&TransferBackMsg { local_channel_id: "source_channel".to_string(), remote_address: "orai123".to_string(), remote_denom: "oraib0x123".to_string(), timeout:None, memo: Some("oraib0x12".to_string()) })? })?, funds: vec![] } 
            .into(),
            gas_limit: None,
            reply_on: cosmwasm_std::ReplyOn::Never,
        }],
        expected_error_string: "".to_string(),
    };
    "Happy Path with cw20 token")]
#[test_case(
    Params {
        caller: "random".to_string(),
        ibc_adapter_contract_address: Addr::unchecked("ibc_transfer".to_string()),
        asset: Asset::Native(Coin::new(100, "osmo")),
        ibc_wasm_info: TransferBackMsg { local_channel_id: "source_channel".to_string(), remote_address: "orai123".to_string(), remote_denom: "oraib0x123".to_string(), timeout:None, memo: Some("oraib0x12".to_string()) },
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

    

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;
    IBC_WASM_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("ibc_wasm"))?;
    // Call execute_ibc_transfer with the given test parameters
    let res = match &params.asset {
        
        Asset::Cw20(native) =>{
            let info = mock_info(&native.address, &[]);
            skip_api_ibc_adapter_orai_ibc_wasm::contract::execute(
                deps.as_mut(),
                env,
                info,
                ExecuteMsg::Receive(Cw20ReceiveMsg { sender: params.caller, amount: native.amount, msg:  to_json_binary(&ExecuteMsg::IbcWasmTransfer { ibc_wasm_info: params.ibc_wasm_info, coin: params.asset })? })
            )
        },
        Asset::Native(coin) => {
            // Create mock info
            let info = mock_info(&params.caller, &[coin.clone()]);
            skip_api_ibc_adapter_orai_ibc_wasm::contract::execute(
                deps.as_mut(),
                env,
                info,
                ExecuteMsg::IbcWasmTransfer { ibc_wasm_info: params.ibc_wasm_info, coin: params.asset }
            )
        }
    };

   

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
