use crate::{
    error::{ContractError, ContractResult},
    state::{
        ACK_ID_TO_RECOVER_ADDRESS, ENTRY_POINT_CONTRACT_ADDRESS, IN_PROGRESS_CHANNEL_ID,
        IN_PROGRESS_RECOVER_ADDRESS,
    },
};
use alloy_sol_types::SolType;
use cosmwasm_std::{
    ensure_eq, entry_point, from_json, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps,
    DepsMut, Env, IbcAckCallbackMsg, IbcBasicResponse, IbcDestinationCallbackMsg, IbcPacket,
    IbcSourceCallbackMsg, IbcTimeoutCallbackMsg, MessageInfo, Reply, Response, StdError, StdResult,
    SubMsg, SubMsgResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use ibc_eureka_solidity_types::msgs::IICS20TransferMsgs::FungibleTokenPacketData as AbiFungibleTokenPacketData;
use ibc_proto::ibc::applications::transfer::v1::{
    FungibleTokenPacketData, MsgTransfer, MsgTransferResponse,
};
use prost::Message;
use serde_cw_value::Value;
use sha2::{Digest, Sha256};
use skip2::{
    callbacks::SourceCallbackType,
    ibc::{AckID, ExecuteMsg, IbcInfo, InstantiateMsg, Memo, MigrateMsg, QueryMsg},
    proto_coin::ProtoCoin,
};
use std::{collections::BTreeMap, str::FromStr};

const IBC_MSG_TRANSFER_TYPE_URL: &str = "/ibc.applications.transfer.v1.MsgTransfer";
const REPLY_ID: u64 = 1;

///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

/////////////////
// INSTANTIATE //
/////////////////

// Contract name and version used for migration.
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate entry point contract address
    let checked_entry_point_contract_address =
        deps.api.addr_validate(&msg.entry_point_contract_address)?;

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.storage, &checked_entry_point_contract_address)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute(
            "entry_point_contract_address",
            checked_entry_point_contract_address.to_string(),
        ))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::IbcTransfer {
            info: ibc_info,
            coin,
            timeout_timestamp,
        } => execute_ibc_transfer(deps, env, info, ibc_info, coin, timeout_timestamp),
    }
}

// Converts the given info and coin into an ibc transfer message,
// saves necessary info in case the ibc transfer fails to send funds back to
// a recovery address, and then emits the ibc transfer message as a sub message
fn execute_ibc_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ibc_info: IbcInfo,
    coin: Coin,
    timeout_timestamp: u64,
) -> ContractResult<Response> {
    // Get entry point contract address from storage
    let entry_point_contract_address = ENTRY_POINT_CONTRACT_ADDRESS.load(deps.storage)?;

    // Enforce the caller is the entry point contract
    if info.sender != entry_point_contract_address {
        return Err(ContractError::Unauthorized);
    }

    // Save in progress recover address to storage, to be used in sudo handler
    IN_PROGRESS_RECOVER_ADDRESS.save(
        deps.storage,
        &ibc_info.recover_address, // This address is verified in entry point
    )?;

    // Save in progress channel id to storage, to be used in sudo handler
    IN_PROGRESS_CHANNEL_ID.save(deps.storage, &ibc_info.source_channel)?;

    // Verify memo is valid json and add the necessary key/value pair to trigger the ibc hooks callback logic.
    let memo = verify_and_create_memo(ibc_info.memo, env.contract.address.to_string())?;

    // If the encoding is None, set it to "" which is treated as classic encoding
    let encoding = ibc_info.encoding.unwrap_or_default();

    // Create an ibc transfer message
    let msg = MsgTransfer {
        source_port: "transfer".to_string(),
        source_channel: ibc_info.source_channel,
        token: Some(ProtoCoin(coin).into()),
        sender: env.contract.address.to_string(),
        receiver: ibc_info.receiver,
        timeout_height: None,
        timeout_timestamp,
        memo,
        encoding,
    };

    // Create stargate message from the ibc transfer message
    let msg = CosmosMsg::Stargate {
        type_url: IBC_MSG_TRANSFER_TYPE_URL.to_string(),
        value: msg.encode_to_vec().into(),
    };

    // Create sub message from the ibc transfer message to receive a reply
    let sub_msg = SubMsg::reply_on_success(msg, REPLY_ID);

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "execute_ibc_transfer"))
}

/////////////
/// REPLY ///
/////////////

// Handles the reply from the ibc transfer sub message
// Upon success, maps the sub msg AckID (channel_id, sequence_id)
// to the in progress ibc transfer struct, and saves it to storage.
// Now that the map entry is stored, it also removes the in progress
// ibc transfer from storage.
#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> ContractResult<Response> {
    // Error if the reply id is not the same as the one used in the sub message dispatched
    // This should never happen since we are using a constant reply id, but added in case
    // the wasm module doesn't behave as expected.
    if reply.id != REPLY_ID {
        unreachable!()
    }

    // Get the sub message response from the reply and error if it does not exist
    // This should never happen since sub msg was set to reply on success only,
    // but added in case the wasm module doesn't behave as expected.
    let SubMsgResult::Ok(sub_msg_response) = reply.result else {
        unreachable!()
    };

    // Parse the response from the sub message
    let resp: MsgTransferResponse = MsgTransferResponse::decode(
        sub_msg_response
            .data
            .ok_or(ContractError::MissingResponseData)?
            .as_slice(),
    )?;

    // Get and delete the in progress recover address from storage
    let in_progress_recover_address = IN_PROGRESS_RECOVER_ADDRESS.load(deps.storage)?;
    IN_PROGRESS_RECOVER_ADDRESS.remove(deps.storage);

    // Get and delete the in progress channel id from storage
    let in_progress_channel_id = IN_PROGRESS_CHANNEL_ID.load(deps.storage)?;
    IN_PROGRESS_CHANNEL_ID.remove(deps.storage);

    // Set ack_id to be the channel id and sequence id from the response as a tuple
    let ack_id: AckID = (&in_progress_channel_id, resp.sequence);

    // Error if unique ack_id (channel id, sequence id) already exists in storage
    if ACK_ID_TO_RECOVER_ADDRESS.has(deps.storage, ack_id) {
        return Err(ContractError::AckIDAlreadyExists {
            channel_id: ack_id.0.into(),
            sequence_id: ack_id.1,
        });
    }

    // Set the in progress recover address to storage, keyed by channel id and sequence id
    ACK_ID_TO_RECOVER_ADDRESS.save(deps.storage, ack_id, &in_progress_recover_address)?;

    Ok(Response::new().add_attribute("action", "sub_msg_reply_success"))
}

#[entry_point]
pub fn ibc_destination_callback(
    _deps: DepsMut,
    _env: Env,
    msg: IbcDestinationCallbackMsg,
) -> ContractResult<IbcBasicResponse> {
    // Require that the packet was sent to the transfer port
    ensure_eq!(
        msg.packet.dest.port_id,
        "transfer",
        StdError::generic_err("only want to handle transfer packets")
    );

    // Require that the packet was successfully received
    // TODO: This fails due to the msg.ack.data having extra bytes then just the
    // scucess ack. Leaving out for now as may be fine.
    // To think more on if we have to verify success here.
    // ensure_eq!(
    //     msg.ack.data,
    //     StdAck::success(b"\x01").to_binary(),
    //     StdError::generic_err("only want to handle successful transfers")
    // );

    // Create the response
    let mut response = IbcBasicResponse::new().add_attribute("action", "ibc_destination_callback");

    // Get the packet data
    let packet_data = get_fungible_token_packet_data(msg.packet.data.clone())?;

    // Get this chain's denom for the packet
    let recv_denom = get_recv_denom(msg.packet, packet_data.denom.clone());

    // Decode the memo to get the contract address and message to execute
    let (contract_addr, msg) = get_contract_addr_and_msg_from_ibc_hooks_memo(packet_data.memo)?;

    // Create a coin to send to the contract based on the packet data
    let coin = Coin {
        denom: recv_denom.clone(),
        amount: Uint128::from_str(&recv_denom)?,
    };

    // @NotJeremyLiu TODO: Turn this into a sub msg and figure out what the recovery case is here
    // Execute the message
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg,
        funds: vec![coin],
    });

    // Add the message to the response
    response = response.add_message(msg);

    // Return the response
    Ok(response)
}

#[entry_point]
pub fn ibc_source_callback(
    deps: DepsMut,
    env: Env,
    msg: IbcSourceCallbackMsg,
) -> ContractResult<IbcBasicResponse> {
    // Get the channel id, sequence id, and source callback type from the message
    let (channel, sequence, callback_type) = match msg {
        IbcSourceCallbackMsg::Acknowledgement(IbcAckCallbackMsg {
            acknowledgement,
            original_packet,
            relayer: _,
            ..
        }) => {
            // Remove the AckID <> in progress ibc transfer from storage
            // and return immediately if the ibc transfer was successful
            // since no further action is needed.
            let ack_str = String::from_utf8_lossy(&acknowledgement.data);
            if ack_str.contains("{\"result\":\"AQ==\"}") {
                let ack_id: AckID = (&original_packet.src.channel_id, original_packet.sequence);
                ACK_ID_TO_RECOVER_ADDRESS.remove(deps.storage, ack_id);

                return Ok(
                    IbcBasicResponse::new().add_attribute("action", SourceCallbackType::Response)
                );
            }

            (
                original_packet.src.channel_id,
                original_packet.sequence,
                SourceCallbackType::Error,
            )
        }
        IbcSourceCallbackMsg::Timeout(IbcTimeoutCallbackMsg {
            packet, relayer: _, ..
        }) => (
            packet.src.channel_id,
            packet.sequence,
            SourceCallbackType::Timeout,
        ),
    };

    // Get and remove the AckID <> in progress recover address from storage
    let ack_id: AckID = (&channel, sequence);
    let to_address = ACK_ID_TO_RECOVER_ADDRESS.load(deps.storage, ack_id)?;
    ACK_ID_TO_RECOVER_ADDRESS.remove(deps.storage, ack_id);

    // Get all coins from contract's balance, which will be the
    // failed ibc transfer coin and any leftover dust on the contract
    let amount = deps.querier.query_all_balances(env.contract.address)?;

    // If amount is empty, return a no funds to refund error
    if amount.is_empty() {
        return Err(ContractError::NoFundsToRefund);
    }

    // Create bank send message to send funds back to user's recover address
    let bank_send_msg = BankMsg::Send { to_address, amount };

    Ok(IbcBasicResponse::new()
        .add_message(bank_send_msg)
        .add_attribute("action", callback_type))
}

//////////////////////
// HELPER FUNCTIONS //
//////////////////////

// Verifies the given memo is empty or valid json, and then adds the necessary
// key/value pair to trigger the src hooks callback logic.
fn verify_and_create_memo(memo: String, contract_address: String) -> ContractResult<String> {
    // If the memo given is empty, then set it to "{}" to avoid json parsing errors. Then,
    // get Value object from json string, erroring if the memo was not null while not being valid json
    let memo_str = if memo.is_empty() { "{}" } else { &memo };

    // Parse the memo into a `Value`, ensuring it's a JSON object (Value::Map).
    let mut memo_map = match serde_json_wasm::from_str::<Value>(memo_str)
        .map_err(|e| StdError::generic_err(format!("Error parsing memo: {e}")))?
    {
        Value::Map(m) => m,
        _ => return Err(ContractError::Std(StdError::generic_err("Invalid memo"))),
    };

    // Create and insert the "src_callback" map containing only the "address".
    let mut src_callback_map = BTreeMap::new();
    src_callback_map.insert(
        Value::String("address".into()),
        Value::String(contract_address),
    );
    memo_map.insert(
        Value::String("src_callback".into()),
        Value::Map(src_callback_map),
    );

    // Serialize the updated memo back to JSON.
    let updated_memo = serde_json_wasm::to_string(&Value::Map(memo_map))
        .map_err(|e| StdError::generic_err(format!("Error serializing memo: {e}")))?;

    Ok(updated_memo)
}

// Parses the given memo string to get the contract address and message to execute,
// decoding the ibc hooks memo
fn get_contract_addr_and_msg_from_ibc_hooks_memo(memo: String) -> StdResult<(String, Binary)> {
    // Convert the memo string to an IBC Hooks Memo struct
    let hooks_memo: Memo = from_json(&memo)?;

    // Return the contract address and the msg as a binary
    Ok((
        hooks_memo.wasm.contract,
        to_json_binary(&hooks_memo.wasm.msg)?,
    ))
}

fn get_fungible_token_packet_data(packet_data: Binary) -> ContractResult<FungibleTokenPacketData> {
    // Try to parse the packet data as a JSON encoded FungibleTokenPacketData
    if let Ok(ftpd) = from_json::<FungibleTokenPacketData>(&packet_data) {
        return Ok(ftpd);
    }

    // Try to parse the packet data as an ABI encoded FungibleTokenPacketData
    if let Ok(ftpd) = AbiFungibleTokenPacketData::abi_decode(&packet_data, true) {
        // Convert the ABI encoded FungibleTokenPacketData to a protobuf encoded FungibleTokenPacketData
        let ftpd_proto = FungibleTokenPacketData {
            denom: ftpd.denom,
            amount: ftpd.amount.to_string(),
            sender: ftpd.sender,
            receiver: ftpd.receiver,
            memo: ftpd.memo,
        };
        return Ok(ftpd_proto);
    }

    // Try to parse the packet data as a protobuf encoded FungibleTokenPacketData
    if let Ok(ftpd) = FungibleTokenPacketData::decode(&*packet_data) {
        return Ok(ftpd);
    }

    Err(ContractError::FailedToDecodePacketData)
}

fn get_recv_denom(ibc_packet: IbcPacket, packet_denom: String) -> String {
    let prefix = format!("{}/{}/", ibc_packet.src.port_id, ibc_packet.src.channel_id);

    // Check if the packet is returning to the origin
    // If so return the original denom
    let is_returning_to_origin = packet_denom.starts_with(&prefix);
    if is_returning_to_origin {
        let ibc_denom = packet_denom.trim_start_matches(&prefix).to_string();
        return ibc_denom;
    }

    // Create and return the proper ibc denom
    let ibc_denom = format!(
        "{}/{}/{packet_denom}",
        ibc_packet.dest.port_id, ibc_packet.dest.channel_id
    );
    let denom_hash = Sha256::digest(ibc_denom.as_bytes()).to_vec();
    format!("ibc/{}", hex::encode(denom_hash))
}

/////////////
/// QUERY ///
/////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::InProgressRecoverAddress {
            channel_id,
            sequence_id,
        } => to_json_binary(
            &ACK_ID_TO_RECOVER_ADDRESS.load(deps.storage, (&channel_id, sequence_id))?,
        ),
    }
    .map_err(From::from)
}

// Scratch Tests

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use alloy_sol_types::SolType;
//     use cosmwasm_std::{from_base64, from_json, IbcTimeout, SubMsgResponse, Timestamp};
//     use ibc_eureka_solidity_types::msgs::IICS20TransferMsgs::FungibleTokenPacketData as AbiFungibleTokenPacketData;
//     use serde_json_wasm::from_slice;

//     use ibc_proto::ibc::applications::transfer::v1::FungibleTokenPacketData;

//     use cosmwasm_std::IbcEndpoint;

//     #[test]
//     fn test_reply() {
//         let sub_msg_result = SubMsgResult::Ok(SubMsgResponse {
//             events: vec![],
//             data: Some(Binary::new(b"11".to_vec())),
//             msg_responses: vec![],
//         });
//         let reply = Reply {
//             id: 1,
//             payload: Binary::default(),
//             gas_used: 0,
//             result: sub_msg_result,
//         };
//         println!("{:#?}", reply);
//         //let resp: MsgTransferResponse = MsgTransferResponse::decode(Binary::new(b"0803".to_vec()).as_slice()).unwrap();
//         let resp: MsgTransferResponse =
//             MsgTransferResponse::decode(Binary::from(vec![0x08, 0x03]).as_slice()).unwrap();
//         println!("{:#?}", resp);
//     }

//     #[test]
//     fn test_get_recv_denom_new_denom() {
//         let timeout = IbcTimeout::with_timestamp(Timestamp::from_nanos(1));
//         let ibc_packet = IbcPacket::new(
//             Binary::default(),
//             IbcEndpoint {
//                 port_id: "transfer".to_string(),
//                 channel_id: "client-6".to_string(),
//             },
//             IbcEndpoint {
//                 port_id: "transfer".to_string(),
//                 channel_id: "08-wasm-0".to_string(),
//             },
//             1,
//             timeout,
//         );

//         let packet_denom = "0xc30edcad074f093882d80424962415cf61494258".to_string();
//         let recv_denom = get_recv_denom(ibc_packet, packet_denom);
//         assert_eq!(
//             recv_denom,
//             "ibc/68afe084b6adf1e1df3867675072f23bf12f270033bd4ce65a10dd7d05d411e1".to_string()
//         );
//     }

//     #[test]
//     fn test_get_recv_denom_returning_denom() {
//         let timeout = IbcTimeout::with_timestamp(Timestamp::from_nanos(1));
//         let ibc_packet = IbcPacket::new(
//             Binary::default(),
//             IbcEndpoint {
//                 port_id: "transfer".to_string(),
//                 channel_id: "client-6".to_string(),
//             },
//             IbcEndpoint {
//                 port_id: "transfer".to_string(),
//                 channel_id: "08-wasm-0".to_string(),
//             },
//             1,
//             timeout,
//         );

//         let packet_denom = "transfer/client-6/stake".to_string();
//         let recv_denom = get_recv_denom(ibc_packet, packet_denom);
//         assert_eq!(recv_denom, "stake".to_string());
//     }

//     #[test]
//     fn test_get_fungible_token_packet_data() {
//         let msg_packet_data_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACoweGMzMGVkY2FkMDc0ZjA5Mzg4MmQ4MDQyNDk2MjQxNWNmNjE0OTQyNTgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACoweDk1MjQ0ZjkwZTExMDQ3YTBkYzA3MjhmMjBjZWZhYzI5ZTZjYTFlYTMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEFjb3Ntb3MxZ2hkNzUzc2hqdXdleHh5d21nczR4ejd4MnE3MzJ2Y25rbTZoMnB5djlzNmFoM2h5bHZycWEwZHI1cQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAYnsiZGVzdF9jYWxsYmFjayI6IHsiYWRkcmVzcyI6ImNvc21vczFnaGQ3NTNzaGp1d2V4eHl3bWdzNHh6N3gycTczMnZjbmttNmgycHl2OXM2YWgzaHlsdnJxYTBkcjVxIn19AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
//         let msg_packet_data: Binary = from_base64(msg_packet_data_str).unwrap().into();

//         let packet_data = get_fungible_token_packet_data(msg_packet_data).unwrap();
//         println!("{:#?}", packet_data);
//     }

//     #[test]
//     fn test_msg_packet_data() {
//         let msg_packet_data_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACoweGMzMGVkY2FkMDc0ZjA5Mzg4MmQ4MDQyNDk2MjQxNWNmNjE0OTQyNTgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACoweDk1MjQ0ZjkwZTExMDQ3YTBkYzA3MjhmMjBjZWZhYzI5ZTZjYTFlYTMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEFjb3Ntb3MxZ2hkNzUzc2hqdXdleHh5d21nczR4ejd4MnE3MzJ2Y25rbTZoMnB5djlzNmFoM2h5bHZycWEwZHI1cQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAYnsiZGVzdF9jYWxsYmFjayI6IHsiYWRkcmVzcyI6ImNvc21vczFnaGQ3NTNzaGp1d2V4eHl3bWdzNHh6N3gycTczMnZjbmttNmgycHl2OXM2YWgzaHlsdnJxYTBkcjVxIn19AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
//         let msg_packet_data: Binary = from_base64(msg_packet_data_str).unwrap().into();
//         // //let packet_data_proto: FungibleTokenPacketData = FungibleTokenPacketData::decode(&*msg_packet_data).unwrap();
//         // // Try to parse the packet data as a JSON encoded FungibleTokenPacketData
//         if let Ok(ftpd) = from_json::<FungibleTokenPacketData>(&msg_packet_data) {
//             println!("Parsed entire thing as JSON!\n{:#?}", ftpd);
//         }

//         // Try to parse the packet data as an ABI encoded FungibleTokenPacketData
//         if let Ok(ftpd) = AbiFungibleTokenPacketData::abi_decode(&msg_packet_data, true) {
//             println!("Parsed entire thing as ABI!");
//             // Print Denom
//             println!("Denom: {}", ftpd.denom);
//             // Print Amount
//             println!("Amount: {}", ftpd.amount);
//             // Print Sender
//             println!("Sender: {}", ftpd.sender);
//             // Print Receiver
//             println!("Receiver: {}", ftpd.receiver);
//             // Print Memo
//             println!("Memo: {}", ftpd.memo);
//         }
//     }

//     #[test]
//     fn test_valid_memo() {
//         // A valid JSON structure containing the fields "wasm" -> { "contract", "msg" }
//         let memo = r#"
//         {
//             "wasm": {
//                 "contract": "some_contract_address",
//                 "msg": {
//                     "some_key": "some_value",
//                     "another_key": 123
//                 }
//             }
//         }
//         "#;

//         let result = get_contract_addr_and_msg_from_ibc_hooks_memo(memo.to_string());
//         assert!(result.is_ok());

//         let (contract_addr, msg_bin) = result.unwrap();
//         assert_eq!(contract_addr, "some_contract_address");

//         // If we want to check the contents of the msg, we can parse the Binary back to JSON.
//         let msg_val: Value = from_slice(&msg_bin).unwrap();
//         // e.g. ensure we got the same structure back
//         let expected_msg =
//             serde_json_wasm::from_str(r#"{ "some_key":"some_value", "another_key":123 }"#).unwrap();
//         assert_eq!(msg_val, expected_msg);
//     }

//     #[test]
//     fn test_valid_ack_data() {
//         let ack_data = StdAck::success(b"\x01").to_binary();
//         println!("{:?}", ack_data);
//         assert_eq!(ack_data, br#"{"result":"AQ=="}"#);
//         println!("{}", ack_data.to_string());

//         let success_result_ack: Binary = br#"{"result":"AQ=="}"#.into();
//         println!("{:?}", success_result_ack);
//         println!("{}", success_result_ack.to_string());
//         let b64_data = success_result_ack.to_base64();
//         println!("{}", b64_data);

//         let b64_data_on_chain = "ChF7InJlc3VsdCI6IkFRPT0ifQ==";
//         let ack_binary = from_base64(b64_data_on_chain).unwrap();
//         println!("{:?}", ack_binary);
//         let ack_str_utf8 = String::from_utf8_lossy(&ack_binary);
//         println!("ack_str_utf8:{}", ack_str_utf8);
//     }
// }
