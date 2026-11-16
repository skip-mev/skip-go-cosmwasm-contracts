use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Binary, Coin, IbcAcknowledgement, IbcDestinationCallbackMsg, IbcEndpoint, IbcPacket,
    IbcTimeout, SubMsg, Timestamp, WasmMsg,
};
use ibc_eureka_solidity_types::msgs::IICS20TransferMsgs::FungibleTokenPacketData as AbiFungibleTokenPacketData;
use ibc_proto::ibc::applications::transfer::v1::FungibleTokenPacketData;
use prost::Message;
use skip_go_ibc_adapter_ibc_callbacks::error::ContractResult;
use test_case::test_case;

/*
Test Cases:

Expect Response (Funds Received By The Adapter Are Forwarded To The Memo Contract)
    - JSON Encoded Packet, Receiver Is The Adapter
    - JSON Encoded Packet, Receiver Is The Adapter In Uppercase
    - ABI Encoded Packet, Receiver Is The Adapter
    - Protobuf Encoded Packet, Receiver Is The Adapter

Expect Error
    - JSON Encoded Packet, Receiver Is Not The Adapter
    - ABI Encoded Packet, Receiver Is Not The Adapter
    - Protobuf Encoded Packet, Receiver Is Not The Adapter
    - Failed Receive Acknowledgement
 */

const ADAPTER: &str = "cosmos1ghd753shjuwexxywmgs4xz7x2q732vcnkm6h2pyv9s6ah3hylvrqa0dr5q";
const MEMO: &str = r#"{"wasm":{"contract":"entry_point","msg":{"swap":{}}}}"#;
const SUCCESS_ACK: &[u8] = br#"{"result":"AQ=="}"#;
// sha256("transfer/channel-1/uosmo")
const RECV_DENOM: &str = "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B";

// Matches ibc-go's wire format (sorted keys), independent of the contract's decoding type
fn json_packet_data(receiver: &str) -> Binary {
    format!(
        r#"{{"amount":"100","denom":"uosmo","memo":{},"receiver":"{receiver}","sender":"osmo1sender"}}"#,
        serde_json_wasm::to_string(MEMO).unwrap()
    )
    .into_bytes()
    .into()
}

// Same layout as the on-chain Eureka packet sample in contract.rs
fn abi_packet_data(receiver: &str) -> Binary {
    AbiFungibleTokenPacketData {
        denom: "uosmo".to_string(),
        sender: "0x95244f90e11047a0dc0728f20cefac29e6ca1ea3".to_string(),
        receiver: receiver.to_string(),
        amount: U256::from(100u64),
        memo: MEMO.to_string(),
    }
    .abi_encode()
    .into()
}

fn proto_packet_data(receiver: &str) -> Binary {
    FungibleTokenPacketData {
        denom: "uosmo".to_string(),
        amount: "100".to_string(),
        sender: "osmo1sender".to_string(),
        receiver: receiver.to_string(),
        memo: MEMO.to_string(),
    }
    .encode_to_vec()
    .into()
}

fn forward_msg() -> Vec<SubMsg> {
    vec![SubMsg::new(WasmMsg::Execute {
        contract_addr: "entry_point".to_string(),
        msg: Binary::from(br#"{"swap":{}}"#),
        funds: vec![Coin::new(100u128, RECV_DENOM)],
    })]
}

// Define test parameters
struct Params {
    packet_data: Binary,
    ack: Binary,
    expected_messages: Vec<SubMsg>,
    expected_error_string: String,
}

// Test ibc_destination_callback
#[test_case(
    Params {
        packet_data: json_packet_data(ADAPTER),
        ack: SUCCESS_ACK.into(),
        expected_messages: forward_msg(),
        expected_error_string: "".to_string(),
    };
    "JSON Encoded Packet, Receiver Is The Adapter")]
#[test_case(
    Params {
        packet_data: json_packet_data(&ADAPTER.to_uppercase()),
        ack: SUCCESS_ACK.into(),
        expected_messages: forward_msg(),
        expected_error_string: "".to_string(),
    };
    "JSON Encoded Packet, Receiver Is The Adapter In Uppercase")]
#[test_case(
    Params {
        packet_data: abi_packet_data(ADAPTER),
        ack: SUCCESS_ACK.into(),
        expected_messages: forward_msg(),
        expected_error_string: "".to_string(),
    };
    "ABI Encoded Packet, Receiver Is The Adapter")]
#[test_case(
    Params {
        packet_data: proto_packet_data(ADAPTER),
        ack: SUCCESS_ACK.into(),
        expected_messages: forward_msg(),
        expected_error_string: "".to_string(),
    };
    "Protobuf Encoded Packet, Receiver Is The Adapter")]
#[test_case(
    Params {
        packet_data: json_packet_data("cosmos1attacker"),
        ack: SUCCESS_ACK.into(),
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
    };
    "JSON Encoded Packet, Receiver Is Not The Adapter - Expect Error")]
#[test_case(
    Params {
        packet_data: abi_packet_data("cosmos1attacker"),
        ack: SUCCESS_ACK.into(),
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
    };
    "ABI Encoded Packet, Receiver Is Not The Adapter - Expect Error")]
#[test_case(
    Params {
        packet_data: proto_packet_data("cosmos1attacker"),
        ack: SUCCESS_ACK.into(),
        expected_messages: vec![],
        expected_error_string: "Unauthorized".to_string(),
    };
    "Protobuf Encoded Packet, Receiver Is Not The Adapter - Expect Error")]
#[test_case(
    Params {
        packet_data: json_packet_data(ADAPTER),
        ack: br#"{"error":"failed"}"#.into(),
        expected_messages: vec![],
        expected_error_string: "Receive packet is not successful, ibc dest callback will not process".to_string(),
    };
    "Failed Receive Acknowledgement - Expect Error")]
fn test_ibc_destination_callback(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env with the adapter address
    let mut env = mock_env();
    env.contract.address = Addr::unchecked(ADAPTER);

    // Create the destination callback message
    let msg = IbcDestinationCallbackMsg {
        packet: IbcPacket::new(
            params.packet_data,
            IbcEndpoint {
                port_id: "transfer".to_string(),
                channel_id: "channel-0".to_string(),
            },
            IbcEndpoint {
                port_id: "transfer".to_string(),
                channel_id: "channel-1".to_string(),
            },
            1,
            IbcTimeout::with_timestamp(Timestamp::from_nanos(1)),
        ),
        ack: IbcAcknowledgement::new(params.ack),
    };

    // Call ibc_destination_callback with the given test parameters
    let res = skip_go_ibc_adapter_ibc_callbacks::contract::ibc_destination_callback(
        deps.as_mut(),
        env,
        msg,
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
