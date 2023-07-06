use crate::ibc::IbcFee;
use cosmwasm_std::{Coin, OverflowError, Uint128};
use std::collections::BTreeMap;

// Coins is a struct that wraps a BTreeMap of String denom to Uint128 total amount
pub struct Coins(BTreeMap<String, Uint128>);

// Converts an IbcFee struct to a Coins struct (BTreeMap<String, Uint128>)
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

// Implement add coin and get amount methods for Coins struct
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

    // Given a denom, returns the total amount of that denom in the Coins struct
    // or returns 0 if the denom is not in the Coins struct.
    pub fn get_amount(&self, denom: &str) -> Uint128 {
        self.0.get(denom).cloned().unwrap_or_default()
    }
}

// Converts a Coins struct to a Vec<Coin>
impl From<Coins> for Vec<Coin> {
    fn from(coins: Coins) -> Self {
        coins
            .0
            .into_iter()
            .map(|(denom, amount)| Coin { denom, amount })
            .collect()
    }
}
