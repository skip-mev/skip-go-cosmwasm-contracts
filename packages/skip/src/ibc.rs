use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, OverflowError, Uint128};
use std::convert::From;

use std::collections::BTreeMap;

use crate::proto_coin::ProtoCoin;
use neutron_proto::neutron::feerefunder::Fee as NeutronFee;

///////////////////
/// INSTANTIATE ///
///////////////////

#[cw_serde]
pub struct InstantiateMsg {}

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
    #[returns(NeutronInProgressIbcTransfer)]
    InProgressIbcTransfer {
        channel_id: String,
        sequence_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum OsmosisQueryMsg {
    #[returns(OsmosisInProgressIbcTransfer)]
    InProgressIbcTransfer {
        channel_id: String,
        sequence_id: u64,
    },
}

////////////////////
/// COMMON TYPES ///
////////////////////

#[cw_serde]
pub struct IbcFee {
    pub recv_fee: Vec<Coin>,
    pub ack_fee: Vec<Coin>,
    pub timeout_fee: Vec<Coin>,
}

// Coins is a type alias for a BTreeMap of String denom to Uint128 total amount
pub struct Coins(BTreeMap<String, Uint128>);

// Converts an IbcFee struct to a BTreeMap of String denom to Uint128 total amount
impl TryFrom<IbcFee> for Coins {
    type Error = OverflowError;

    fn try_from(ibc_fee: IbcFee) -> Result<Self, Self::Error> {
        let mut ibc_fees = Coins(BTreeMap::new());

        for coin in [ibc_fee.recv_fee, ibc_fee.ack_fee, ibc_fee.timeout_fee]
            .iter()
            .flatten()
        {
            ibc_fees.add_coin(coin)?;
        }

        Ok(ibc_fees)
    }
}

// Implement add coin and get amount methods for Coins
impl Coins {
    // Takes a coin and adds it to the Coins map
    pub fn add_coin(&mut self, coin: &Coin) -> Result<(), OverflowError> {
        let amount = self
            .0
            .entry(coin.denom.clone())
            .or_insert_with(Uint128::zero);
        *amount = amount.checked_add(coin.amount)?;

        Ok(())
    }

    // Given a denom, returns the total amount of that denom in the Coins map
    // or returns 0 if the denom is not in the Coins map.
    pub fn get_amount(&self, denom: &str) -> Uint128 {
        self.0.get(denom).cloned().unwrap_or_default()
    }
}

// Converts a Coins map to a Vec<Coin>
impl From<Coins> for Vec<Coin> {
    fn from(coins: Coins) -> Self {
        coins
            .0
            .into_iter()
            .map(|(denom, amount)| Coin { denom, amount })
            .collect()
    }
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

#[cw_serde]
pub struct IbcInfo {
    pub source_channel: String,
    pub receiver: String,
    pub fee: IbcFee,
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

// NeutronInProgressIBCTransfer is a struct that is used to store the ibc transfer information
// upon receiving a successful response from the neutron ibc transfer sub message. Later
// to be used in the sudo handler to send the coin back to the recover address if the
// ibc transfer packet acknowledgement is an error or times out. Also used to send the
// user back the refunded ack fee or timeout fee based on the type of acknowledgement
#[cw_serde]
pub struct NeutronInProgressIbcTransfer {
    pub recover_address: String,
    pub coin: Coin,
    pub ack_fee: Vec<Coin>,
    pub timeout_fee: Vec<Coin>,
}

// OsmosisInProgressIBCTransfer is a struct that is used to store the ibc transfer information
// upon receiving a successful sub message reply. Later
// to be used in the sudo handler to send the coin back to the recover address if the
// ibc transfer packet acknowledgement is an error or times out. Also used to send the
// user back the refunded ack fee or timeout fee based on the type of acknowledgement
#[cw_serde]
pub struct OsmosisInProgressIbcTransfer {
    pub recover_address: String,
    pub coin: Coin,
    pub channel_id: String,
}

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
