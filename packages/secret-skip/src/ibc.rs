use crate::{error::SkipError, snip20::Snip20ReceiveMsg};

use std::convert::From;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, ContractInfo, StdError};

// Used with Secret network ICS20 contract
// for auto wrapping & unrapping of snip20 tokens
// for IBC transfers
#[cw_serde]
pub struct Ics20TransferMsg {
    /// The local channel to send the packets on
    pub channel: String,
    /// The remote address to send to
    /// Don't use HumanAddress as this will likely have a different Bech32 prefix than we use
    /// and cannot be validated locally
    pub remote_address: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
}

///////////////
/// MIGRATE ///
///////////////

// The MigrateMsg struct defines the migration parameters used.
#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract: ContractInfo,
    pub ics20_contract: ContractInfo,
}
///////////////////
/// INSTANTIATE ///
///////////////////

// The InstantiateMsg struct defines the initialization parameters for the IBC Transfer Adapter contracts.
#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract: ContractInfo,
    pub ics20_contract: ContractInfo,
}

///////////////
/// EXECUTE ///
///////////////

// The ExecuteMsg enum defines the execution message that the IBC Transfer Adapter contracts can handle.
#[cw_serde]
pub enum ExecuteMsg {
    IbcTransfer {
        info: IbcInfo,
        coin: Coin,
        timeout_timestamp: u64,
    },
    RegisterTokens {
        contracts: Vec<ContractInfo>,
    },
    Receive(Snip20ReceiveMsg),
}

#[cw_serde]
pub enum Snip20HookMsg {
    IbcTransfer {
        info: IbcInfo,
        timeout_timestamp: u64,
    },
}

/////////////
/// QUERY ///
/////////////

// The QueryMsg enum defines the queries the IBC Transfer Adapter Contract provides.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    InProgressRecoverAddress {
        channel_id: String,
        sequence_id: u64,
    },
}

////////////////////
/// COMMON TYPES ///
////////////////////

// The IbcFee struct defines the fees for an IBC transfer standardized across all IBC Transfer Adapter contracts.
#[cw_serde]
#[derive(Default)]
pub struct IbcFee {
    pub recv_fee: Vec<Coin>,
    pub ack_fee: Vec<Coin>,
    pub timeout_fee: Vec<Coin>,
}

// Converts an IbcFee struct to a cosmwasm_std::Coins struct
// Must be TryFrom since adding the ibc_fees can overflow.
impl TryFrom<IbcFee> for Vec<Coin> {
    type Error = StdError;

    fn try_from(ibc_fee: IbcFee) -> Result<Self, Self::Error> {
        Ok([ibc_fee.recv_fee, ibc_fee.ack_fee, ibc_fee.timeout_fee]
            .into_iter()
            .flatten()
            .collect())
    }
}

impl IbcFee {
    // one_coin aims to mimic the behavior of cw_utls::one_coin,
    // returing the single coin in the IbcFee struct if it exists,
    // erroring if 0 or more than 1 coins exist.
    //
    // one_coin is used because the entry_point contract only supports
    // the handling of a single denomination for IBC fees.
    pub fn one_coin(&self) -> Result<Coin, SkipError> {
        let ibc_fees: Vec<Coin> = self.clone().try_into()?;

        if ibc_fees.len() != 1 {
            return Err(SkipError::IbcFeesNotOneCoin);
        }

        Ok(ibc_fees.first().unwrap().clone())
    }
}

// The IbcInfo struct defines the information for an IBC transfer standardized across all IBC Transfer Adapter contracts.
#[cw_serde]
pub struct IbcInfo {
    pub source_channel: String,
    pub receiver: String,
    pub fee: Option<IbcFee>,
    pub memo: String,
    pub recover_address: String,
}

// The IbcTransfer struct defines the parameters for an IBC transfer standardized across all IBC Transfer Adapter contracts.
#[cw_serde]
pub struct IbcTransfer {
    pub info: IbcInfo,
    pub coin: Coin,
    pub timeout_timestamp: u64,
}

// Converts an IbcTransfer struct to an ExecuteMsg::IbcTransfer enum
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

// The IbcLifecycleComplete enum defines the possible sudo messages that the
// ibc transfer adapter contract on ibc-hook enabled chains can expect to received
// from the ibc-hooks module.
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Uint128;

    #[test]
    fn test_from_ibc_fee_for_neutron_proto_fee() {
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(100, "atom")],
            ack_fee: vec![Coin::new(100, "osmo")],
            timeout_fee: vec![Coin::new(100, "ntrn")],
        };

        let neutron_fee: NeutronFee = ibc_fee.into();

        assert_eq!(neutron_fee.recv_fee.len(), 1);
        assert_eq!(neutron_fee.ack_fee.len(), 1);
        assert_eq!(neutron_fee.timeout_fee.len(), 1);

        assert_eq!(neutron_fee.recv_fee[0].denom, "atom");
        assert_eq!(neutron_fee.recv_fee[0].amount, "100");

        assert_eq!(neutron_fee.ack_fee[0].denom, "osmo");
        assert_eq!(neutron_fee.ack_fee[0].amount, "100");

        assert_eq!(neutron_fee.timeout_fee[0].denom, "ntrn");
        assert_eq!(neutron_fee.timeout_fee[0].amount, "100");
    }

    /*
    #[test]
    fn test_try_from_ibc_fee_for_coins() {
        // TEST CASE 1: Same Denom For All Fees
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(100, "atom")],
            ack_fee: vec![Coin::new(100, "atom")],
            timeout_fee: vec![Coin::new(100, "atom")],
        };

        let coins: Coins = ibc_fee.try_into().unwrap();

        assert_eq!(coins.len(), 1);
        assert_eq!(coins.amount_of("atom"), Uint128::from(300u128));

        // TEST CASE 2: Different Denom For Some Fees
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(100, "atom")],
            ack_fee: vec![Coin::new(100, "osmo")],
            timeout_fee: vec![Coin::new(100, "atom")],
        };

        let coins: Coins = ibc_fee.try_into().unwrap();

        assert_eq!(coins.len(), 2);
        assert_eq!(coins.amount_of("atom"), Uint128::from(200u128));
        assert_eq!(coins.amount_of("osmo"), Uint128::from(100u128));
    }
    */

    #[test]
    fn test_one_coin() {
        // TEST CASE 1: No Coins
        let ibc_fee = IbcFee {
            recv_fee: vec![],
            ack_fee: vec![],
            timeout_fee: vec![],
        };

        let result = ibc_fee.one_coin();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SkipError::IbcFeesNotOneCoin);

        // TEST CASE 2: One Coin
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(100, "atom")],
            ack_fee: vec![],
            timeout_fee: vec![],
        };

        let result = ibc_fee.one_coin();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Coin::new(100, "atom"));

        // TEST CASE 3: More Than One Coin
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(100, "atom")],
            ack_fee: vec![Coin::new(100, "osmo")],
            timeout_fee: vec![],
        };

        let result = ibc_fee.one_coin();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SkipError::IbcFeesNotOneCoin);
    }
}
