use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, IbcMsg, IbcTimeout, StdResult, Uint128};

/// The format for sending an ics20 packet.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/applications/transfer/v1/transfer.proto#L11-L20
/// This is compatible with the JSON serialization
#[cw_serde]
pub struct Ics20Packet {
    /// amount of tokens to transfer is encoded as a string
    pub amount: Uint128,
    /// the token denomination to be transferred
    pub denom: String,
    /// the recipient address on the destination chain
    pub receiver: String,
    /// the sender address
    pub sender: String,
    /// optional memo
    pub memo: Option<String>,
}

impl Ics20Packet {
    pub fn new<T: Into<String>>(
        amount: Uint128,
        denom: T,
        sender: &str,
        receiver: &str,
        memo: Option<String>,
    ) -> Self {
        Ics20Packet {
            denom: denom.into(),
            amount,
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            memo,
        }
    }
}

pub fn build_ibc_send_packet(
    amount: Uint128,
    denom: &str,
    sender: &str,
    receiver: &str,
    memo: Option<String>,
    src_channel: &str,
    timeout: IbcTimeout,
) -> StdResult<IbcMsg> {
    // build ics20 packet
    let packet = Ics20Packet::new(
        amount,
        denom, // we use ibc denom in form <transfer>/<channel>/<denom> so that when it is sent back to remote chain, it gets parsed correctly and burned
        sender, receiver, memo,
    );

    // prepare ibc message
    Ok(IbcMsg::SendPacket {
        channel_id: src_channel.to_string(),
        data: to_json_binary(&packet)?,
        timeout,
    })
}
