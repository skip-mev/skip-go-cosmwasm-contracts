use crate::proto_coin::ProtoCoin;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Coins, StdError};
use neutron_proto::neutron::feerefunder::Fee as NeutronFee;
use std::convert::From;

///////////////////
/// INSTANTIATE ///
///////////////////

#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
}

///////////////
/// EXECUTE ///
///////////////

#[cw_serde]
pub enum ExecuteMsg {
    IbcTransfer {
        info: IbcInfo,
        coin: Coin,
        timeout_timestamp: u64,
    },
}

/////////////
/// QUERY ///
/////////////

#[cw_serde]
#[derive(QueryResponses)]
pub enum NeutronQueryMsg {
    #[returns(String)]
    InProgressRecoverAddress {
        channel_id: String,
        sequence_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum OsmosisQueryMsg {
    #[returns(String)]
    InProgressRecoverAddress {
        channel_id: String,
        sequence_id: u64,
    },
}

////////////////////
/// COMMON TYPES ///
////////////////////

#[cw_serde]
#[derive(Default)]
pub struct IbcFee {
    pub recv_fee: Vec<Coin>,
    pub ack_fee: Vec<Coin>,
    pub timeout_fee: Vec<Coin>,
}

// Converts an IbcFee struct to a neutron_proto Fee
impl From<IbcFee> for NeutronFee {
    fn from(ibc_fee: IbcFee) -> Self {
        NeutronFee {
            recv_fee: ibc_fee
                .recv_fee
                .iter()
                .map(|coin| ProtoCoin(coin.clone()).into())
                .collect(),
            ack_fee: ibc_fee
                .ack_fee
                .iter()
                .map(|coin| ProtoCoin(coin.clone()).into())
                .collect(),
            timeout_fee: ibc_fee
                .timeout_fee
                .iter()
                .map(|coin| ProtoCoin(coin.clone()).into())
                .collect(),
        }
    }
}

// Converts an IbcFee struct to a Coins struct
impl TryFrom<IbcFee> for Coins {
    type Error = StdError;

    fn try_from(ibc_fee: IbcFee) -> Result<Self, Self::Error> {
        let mut ibc_fees = Coins::default();

        [ibc_fee.recv_fee, ibc_fee.ack_fee, ibc_fee.timeout_fee]
            .into_iter()
            .flatten()
            .try_for_each(|coin| ibc_fees.add(coin))?;

        Ok(ibc_fees)
    }
}

#[cw_serde]
pub struct IbcInfo {
    pub source_channel: String,
    pub receiver: String,
    pub fee: Option<IbcFee>,
    pub memo: String,
    pub recover_address: String,
}

#[cw_serde]
pub struct IbcTransfer {
    pub info: IbcInfo,
    pub coin: Coin,
    pub timeout_timestamp: u64,
}

impl From<IbcTransfer> for ExecuteMsg {
    fn from(ibc_transfer: IbcTransfer) -> Self {
        ExecuteMsg::IbcTransfer {
            info: ibc_transfer.info,
            coin: ibc_transfer.coin,
            timeout_timestamp: ibc_transfer.timeout_timestamp,
        }
    }
}

// AckID is a type alias for a tuple of a str and a u64
// which is used as a lookup key to store the in progress
// ibc transfer upon receiving a successful sub msg reply.
pub type AckID<'a> = (&'a str, u64);

#[cw_serde]
pub enum IbcLifecycleComplete {
    IbcAck {
        /// The source channel of the IBC packet
        channel: String,
        /// The sequence number that the packet was sent with
        sequence: u64,
        /// String encoded version of the ack as seen by OnAcknowledgementPacket(..)
        ack: String,
        /// Whether an ack is a success of failure according to the transfer spec
        success: bool,
    },
    IbcTimeout {
        /// The source channel of the IBC packet
        channel: String,
        /// The sequence number that the packet was sent with
        sequence: u64,
    },
}
