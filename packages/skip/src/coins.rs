use crate::{error::SkipError, ibc::IbcFee};
use cosmwasm_std::{Coin, OverflowError, Uint128};
use std::collections::BTreeMap;

// Coins is a struct that wraps a BTreeMap of String denom to Uint128 total amount
pub struct Coins(BTreeMap<String, Uint128>);

// Implement add coin and get amount methods for Coins struct
impl Coins {
    // Create a new Coins struct
    pub fn new() -> Self {
        Coins(BTreeMap::new())
    }

    // Takes a coin and adds it to the Coins map
    pub fn add_coin(&mut self, coin: &Coin) -> Result<(), OverflowError> {
        // Do not allow zero amount coin to create a new entry
        if coin.amount.is_zero() {
            return Ok(());
        }

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

    // Returns true if Coins map is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // Returns the legnth of the map
    pub fn len(&self) -> usize {
        self.0.len()
    }

    // If there exists only one coin in the map, return it, otherwise error
    pub fn one_coin(&self) -> Result<Coin, SkipError> {
        if self.len() != 1 {
            return Err(SkipError::IbcFeesNotOneCoin);
        }

        let (denom, amount) = self.0.first_key_value().unwrap();

        Ok(Coin {
            denom: denom.clone(),
            amount: *amount,
        })
    }

    // Adds the given IbcFee to the Coins struct
    pub fn add_ibc_fee(&mut self, ibc_fee: &IbcFee) -> Result<(), OverflowError> {
        [
            ibc_fee.recv_fee.clone(),
            ibc_fee.ack_fee.clone(),
            ibc_fee.timeout_fee.clone(),
        ]
        .iter()
        .flatten()
        .try_for_each(|coin| self.add_coin(coin))
    }
}

impl Default for Coins {
    fn default() -> Self {
        Self::new()
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

// Converts an IbcFee struct to a Coins struct
impl TryFrom<IbcFee> for Coins {
    type Error = OverflowError;

    fn try_from(ibc_fee: IbcFee) -> Result<Self, Self::Error> {
        let mut ibc_fees = Coins(BTreeMap::new());

        [ibc_fee.recv_fee, ibc_fee.ack_fee, ibc_fee.timeout_fee]
            .iter()
            .flatten()
            .try_for_each(|coin| ibc_fees.add_coin(coin))?;

        Ok(ibc_fees)
    }
}
