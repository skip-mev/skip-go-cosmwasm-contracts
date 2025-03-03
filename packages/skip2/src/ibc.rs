use std::convert::From;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Coins, StdError};
use serde_cw_value::Value;

///////////////
/// MIGRATE ///
///////////////

// The MigrateMsg struct defines the migration parameters used.
#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract_address: String,
}
/////////////////
// INSTANTIATE //
/////////////////

// The InstantiateMsg struct defines the initialization parameters for the IBC Transfer Adapter contracts.
#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
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
#[derive(Default)]
pub struct EurekaFee {
    pub coin: Coin,
    pub receiver: String,
    pub timeout_timestamp: u64,
}

// The IbcInfo struct defines the information for an IBC transfer standardized across all IBC Transfer Adapter contracts.
#[cw_serde]
pub struct IbcInfo {
    pub source_channel: String,
    pub receiver: String,
    pub fee: Option<IbcFee>,
    pub memo: String,
    pub recover_address: String,
    pub encoding: Option<String>,
    pub eureka_fee: Option<EurekaFee>,
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

/// Top-level memo struct
#[cw_serde]
pub struct Memo {
    pub wasm: WasmData,
}

/// Nested "wasm" object
#[cw_serde]
pub struct WasmData {
    pub contract: String,

    #[schemars(skip)]
    pub msg: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Uint128;

    #[test]
    fn test_try_from_ibc_fee_for_coins() {
        // TEST CASE 1: Same Denom For All Fees
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(Uint128::new(100), "atom")],
            ack_fee: vec![Coin::new(Uint128::new(100), "atom")],
            timeout_fee: vec![Coin::new(Uint128::new(100), "atom")],
        };

        let coins: Coins = ibc_fee.try_into().unwrap();

        assert_eq!(coins.len(), 1);
        assert_eq!(coins.amount_of("atom"), Uint128::from(300u128));

        // TEST CASE 2: Different Denom For Some Fees
        let ibc_fee = IbcFee {
            recv_fee: vec![Coin::new(Uint128::new(100), "atom")],
            ack_fee: vec![Coin::new(Uint128::new(100), "osmo")],
            timeout_fee: vec![Coin::new(Uint128::new(100), "atom")],
        };

        let coins: Coins = ibc_fee.try_into().unwrap();

        assert_eq!(coins.len(), 2);
        assert_eq!(coins.amount_of("atom"), Uint128::from(200u128));
        assert_eq!(coins.amount_of("osmo"), Uint128::from(100u128));
    }
}
