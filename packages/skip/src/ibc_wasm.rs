use crate::error::SkipError;

use std::convert::From;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Coins, StdError};

///////////////
/// MIGRATE ///
///////////////

// The MigrateMsg struct defines the migration parameters used.
#[cw_serde]
pub struct MigrateMsg {
    pub entry_point_contract_address: String,
    pub ibc_wasm_contract_address: String,
}
///////////////////
/// INSTANTIATE ///
///////////////////

// The InstantiateMsg struct defines the initialization parameters for the IBC Transfer Adapter contracts.
#[cw_serde]
pub struct InstantiateMsg {
    pub entry_point_contract_address: String,
    pub ibc_wasm_contract_address: String,
}

///////////////
/// EXECUTE ///
///////////////

// The ExecuteMsg enum defines the execution message that the IBC Transfer Adapter contracts can handle.
#[cw_serde]
pub enum ExecuteMsg {
    IbcWasmTransfer {
        info: IbcWasmInfo,
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
pub enum QueryMsg {}

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

impl IbcFee {
    // one_coin aims to mimic the behavior of cw_utls::one_coin,
    // returing the single coin in the IbcFee struct if it exists,
    // erroring if 0 or more than 1 coins exist.
    //
    // one_coin is used because the entry_point contract only supports
    // the handling of a single denomination for IBC fees.
    pub fn one_coin(&self) -> Result<Coin, SkipError> {
        let ibc_fees_map: Coins = self.clone().try_into()?;

        if ibc_fees_map.len() != 1 {
            return Err(SkipError::IbcFeesNotOneCoin);
        }

        Ok(ibc_fees_map.to_vec().first().unwrap().clone())
    }
}

// The IbcWasmInfo struct defines the information for an IBC transfer standardized across all IBC WASM Transfer Adapter contracts.
#[cw_serde]
pub struct IbcWasmInfo {
    pub local_channel_id: String,
    pub remote_address: String,
    pub remote_denom: String, // prefix + erc20
    pub memo: String,         // prefix + evm address
    pub timeout: Option<u64>,
    pub fee: Option<IbcFee>,
}

// The IbcTransfer struct defines the parameters for an IBC transfer standardized across all IBC Transfer Adapter contracts.
#[cw_serde]
pub struct IbcWasmTransfer {
    pub info: IbcWasmInfo,
    pub coin: Coin,
    pub timeout_timestamp: u64,
}

// Converts an IbcTransfer struct to an ExecuteMsg::IbcTransfer enum
impl From<IbcWasmTransfer> for ExecuteMsg {
    fn from(ibc_transfer: IbcWasmTransfer) -> Self {
        ExecuteMsg::IbcWasmTransfer {
            info: ibc_transfer.info,
            coin: ibc_transfer.coin,
            timeout_timestamp: ibc_transfer.timeout_timestamp,
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::Uint128;

//     #[test]
//     fn test_from_ibc_fee_for_neutron_proto_fee() {
//         let ibc_fee = IbcFee {
//             recv_fee: vec![Coin::new(100, "atom")],
//             ack_fee: vec![Coin::new(100, "osmo")],
//             timeout_fee: vec![Coin::new(100, "ntrn")],
//         };

//         let neutron_fee: NeutronFee = ibc_fee.into();

//         assert_eq!(neutron_fee.recv_fee.len(), 1);
//         assert_eq!(neutron_fee.ack_fee.len(), 1);
//         assert_eq!(neutron_fee.timeout_fee.len(), 1);

//         assert_eq!(neutron_fee.recv_fee[0].denom, "atom");
//         assert_eq!(neutron_fee.recv_fee[0].amount, "100");

//         assert_eq!(neutron_fee.ack_fee[0].denom, "osmo");
//         assert_eq!(neutron_fee.ack_fee[0].amount, "100");

//         assert_eq!(neutron_fee.timeout_fee[0].denom, "ntrn");
//         assert_eq!(neutron_fee.timeout_fee[0].amount, "100");
//     }

//     #[test]
//     fn test_try_from_ibc_fee_for_coins() {
//         // TEST CASE 1: Same Denom For All Fees
//         let ibc_fee = IbcFee {
//             recv_fee: vec![Coin::new(100, "atom")],
//             ack_fee: vec![Coin::new(100, "atom")],
//             timeout_fee: vec![Coin::new(100, "atom")],
//         };

//         let coins: Coins = ibc_fee.try_into().unwrap();

//         assert_eq!(coins.len(), 1);
//         assert_eq!(coins.amount_of("atom"), Uint128::from(300u128));

//         // TEST CASE 2: Different Denom For Some Fees
//         let ibc_fee = IbcFee {
//             recv_fee: vec![Coin::new(100, "atom")],
//             ack_fee: vec![Coin::new(100, "osmo")],
//             timeout_fee: vec![Coin::new(100, "atom")],
//         };

//         let coins: Coins = ibc_fee.try_into().unwrap();

//         assert_eq!(coins.len(), 2);
//         assert_eq!(coins.amount_of("atom"), Uint128::from(200u128));
//         assert_eq!(coins.amount_of("osmo"), Uint128::from(100u128));
//     }

//     #[test]
//     fn test_one_coin() {
//         // TEST CASE 1: No Coins
//         let ibc_fee = IbcFee {
//             recv_fee: vec![],
//             ack_fee: vec![],
//             timeout_fee: vec![],
//         };

//         let result = ibc_fee.one_coin();

//         assert!(result.is_err());
//         assert_eq!(result.unwrap_err(), SkipError::IbcFeesNotOneCoin);

//         // TEST CASE 2: One Coin
//         let ibc_fee = IbcFee {
//             recv_fee: vec![Coin::new(100, "atom")],
//             ack_fee: vec![],
//             timeout_fee: vec![],
//         };

//         let result = ibc_fee.one_coin();

//         assert!(result.is_ok());
//         assert_eq!(result.unwrap(), Coin::new(100, "atom"));

//         // TEST CASE 3: More Than One Coin
//         let ibc_fee = IbcFee {
//             recv_fee: vec![Coin::new(100, "atom")],
//             ack_fee: vec![Coin::new(100, "osmo")],
//             timeout_fee: vec![],
//         };

//         let result = ibc_fee.one_coin();

//         assert!(result.is_err());
//         assert_eq!(result.unwrap_err(), SkipError::IbcFeesNotOneCoin);
//     }
// }
