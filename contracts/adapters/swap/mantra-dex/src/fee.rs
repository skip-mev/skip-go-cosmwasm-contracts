use std::fmt::{Display, Formatter};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Decimal256, StdError, StdResult, Uint128, Uint256};

#[cw_serde]
pub struct Fee {
    pub share: Decimal,
}

impl Fee {
    /// Computes the fee for the given amount
    pub fn compute(&self, amount: Uint256) -> StdResult<Uint256> {
        Ok(Decimal256::from_ratio(amount, Uint256::one())
            .checked_mul(self.to_decimal_256())
            .map_err(|e| StdError::generic_err(e.to_string()))?
            .to_uint_floor())
    }

    /// Converts a Fee to a Decimal256
    pub fn to_decimal_256(&self) -> Decimal256 {
        Decimal256::from(self.share)
    }

    /// Checks that the given [Fee] is valid, i.e. it's lower or equal to 100%
    pub fn is_valid(&self) -> StdResult<()> {
        if self.share >= Decimal::percent(100) {
            return Err(StdError::generic_err("Invalid fee"));
        }
        Ok(())
    }
}

impl Display for Fee {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.share * Decimal::percent(100))
    }
}

/// Represents the fee structure for transactions within a pool.
///
///
/// # Fields
/// - `protocol_fee`: The fee percentage charged by the protocol on each transaction to support
///   operational and developmental needs.
/// - `swap_fee`: The fee percentage allocated to liquidity providers as a reward for supplying
///   liquidity to the pool, incentivizing participation and ensuring pool health.
/// - `burn_fee`: A fee percentage that is burned on each transaction, helping manage the token
///   economy by reducing supply over time, potentially increasing token value.
/// - `extra_fees`: A vector of custom fees allowing for extensible and adaptable fee structures
///   to meet diverse and evolving needs. Validation ensures that the total of all fees does not
///   exceed 100%, maintaining fairness and avoiding overcharging.
#[cw_serde]
pub struct PoolFee {
    /// Fee percentage charged on each transaction for the protocol's benefit.
    pub protocol_fee: Fee,

    /// Fee percentage allocated to liquidity providers on each swap.
    pub swap_fee: Fee,

    /// Fee percentage that is burned on each transaction. Burning a portion of the transaction fee
    /// helps in reducing the overall token supply.
    pub burn_fee: Fee,

    /// A list of custom, additional fees that can be defined for specific use cases or additional
    /// functionalities. This vector enables the flexibility to introduce new fees without altering
    /// the core fee structure. Total of all fees, including custom ones, is validated to not exceed
    /// 100%, ensuring a balanced and fair fee distribution.
    pub extra_fees: Vec<Fee>,
}

impl PoolFee {
    /// Validates the PoolFee structure to ensure the sum of all fees does not exceed 20%.
    pub fn is_valid(&self) -> StdResult<()> {
        let mut total_share = Decimal::zero();

        // Validate predefined fees and accumulate their shares
        let predefined_fees = [&self.protocol_fee, &self.swap_fee, &self.burn_fee];

        for fee in predefined_fees.iter().copied() {
            fee.is_valid()?; // Validates the fee is not >= 100%
            total_share += fee.share;
        }

        // Validate extra fees and accumulate their shares
        for fee in &self.extra_fees {
            fee.is_valid()?; // Validates the fee is not >= 100%
            total_share += fee.share;
        }

        // Check if the total share exceeds 20%
        if total_share > Decimal::percent(20) {
            return Err(StdError::generic_err("Total fees cannot exceed 20%"));
        }

        Ok(())
    }

    /// Computes and applies all defined fees to a given amount.
    /// Returns the total amount of fees deducted.
    pub fn compute_and_apply_fees(&self, amount: Uint256) -> StdResult<Uint128> {
        let mut total_fee_amount = Uint256::zero();

        // Compute protocol fee
        let protocol_fee_amount = self.protocol_fee.compute(amount)?;
        total_fee_amount = total_fee_amount.checked_add(protocol_fee_amount)?;

        // Compute swap fee
        let swap_fee_amount = self.swap_fee.compute(amount)?;
        total_fee_amount = total_fee_amount.checked_add(swap_fee_amount)?;

        // Compute burn fee
        let burn_fee_amount = self.burn_fee.compute(amount)?;
        total_fee_amount = total_fee_amount.checked_add(burn_fee_amount)?;

        // Compute extra fees
        for extra_fee in &self.extra_fees {
            let extra_fee_amount = extra_fee.compute(amount)?;
            total_fee_amount = total_fee_amount.checked_add(extra_fee_amount)?;
        }

        // Convert the total fee amount to Uint128 (or handle potential conversion failure)
        Uint128::try_from(total_fee_amount)
            .map_err(|_| StdError::generic_err("Fee conversion error"))
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, StdError, Uint128, Uint256};
    use test_case::test_case;

    use crate::fee::{Fee, PoolFee};

    #[test]
    fn valid_fee() {
        let fee = Fee {
            share: Decimal::from_ratio(9u128, 10u128),
        };
        let res = fee.is_valid();
        match res {
            Ok(_) => (),
            Err(_) => panic!("this fee shouldn't fail"),
        }

        let fee = Fee {
            share: Decimal::from_ratio(Uint128::new(2u128), Uint128::new(100u128)),
        };
        let res = fee.is_valid();
        match res {
            Ok(_) => (),
            Err(_) => panic!("this fee shouldn't fail"),
        }

        let fee = Fee {
            share: Decimal::zero(),
        };
        let res = fee.is_valid();
        match res {
            Ok(_) => (),
            Err(_) => panic!("this fee shouldn't fail"),
        }
    }

    #[test]
    fn invalid_fee() {
        let fee = Fee {
            share: Decimal::one(),
        };
        assert_eq!(fee.is_valid(), Err(StdError::generic_err("Invalid fee")));

        let fee = Fee {
            share: Decimal::from_ratio(Uint128::new(2u128), Uint128::new(1u128)),
        };
        assert_eq!(fee.is_valid(), Err(StdError::generic_err("Invalid fee")));
    }

    #[test_case(
        Decimal::permille(1), Decimal::permille(2), Decimal::permille(1), Uint256::from(1000u128), Uint128::from(4u128); "low fee scenario"
    )]
    #[test_case(
        Decimal::percent(1), Decimal::percent(2), Decimal::zero(), Uint256::from(1000u128), Uint128::from(30u128); "higher fee scenario"
    )]
    fn pool_fee_application(
        protocol_fee_share: Decimal,
        swap_fee_share: Decimal,
        burn_fee_share: Decimal,
        amount: Uint256,
        expected_fee_deducted: Uint128,
    ) {
        let protocol_fee = Fee {
            share: protocol_fee_share,
        };
        let swap_fee = Fee {
            share: swap_fee_share,
        };
        let burn_fee = Fee {
            share: burn_fee_share,
        };
        let extra_fees = vec![]; // Assuming no extra fees for simplicity

        let pool_fee = PoolFee {
            protocol_fee,
            swap_fee,
            burn_fee,
            extra_fees,
        };

        let total_fee_deducted = pool_fee.compute_and_apply_fees(amount).unwrap();
        assert_eq!(
            total_fee_deducted, expected_fee_deducted,
            "The total deducted fees did not match the expected value."
        );
    }

    #[test]
    fn pool_fee_exceeds_limit() {
        let protocol_fee = Fee {
            share: Decimal::percent(10),
        };
        let swap_fee = Fee {
            share: Decimal::percent(5),
        };
        let burn_fee = Fee {
            share: Decimal::percent(5),
        };
        let extra_fees = vec![Fee {
            share: Decimal::percent(1),
        }]; // Sum is 21%

        let pool_fee = PoolFee {
            protocol_fee,
            swap_fee,
            burn_fee,
            extra_fees,
        };

        assert_eq!(
            pool_fee.is_valid(),
            Err(StdError::generic_err("Total fees cannot exceed 20%"))
        );
    }
}
