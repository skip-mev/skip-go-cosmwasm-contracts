//! Message types of the wasmswap pool contract, plus price math.
//!
//! The interface was recovered from the live contract on Axiome (variant lists come
//! from the contract's own parse errors, field names confirmed by simulation):
//!   QueryMsg:   balance, info, token1_for_token2_price, token2_for_token1_price, fee
//!   ExecuteMsg: add_liquidity, remove_liquidity, swap, pass_through_swap,
//!               swap_and_send_to, update_config, freeze_deposits
//!
//! Important difference from astroport/white-whale: wasmswap has NO cw20 Send hook.
//! A cw20 input goes through increase_allowance + swap (the pool performs the
//! TransferFrom itself).

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128, Uint256};
use cw20::Expiration;

/// Fee scale used by wasmswap: percentages are converted to basis points.
const FEE_SCALE_FACTOR: u128 = 10_000;
/// Decimal stores 18 fractional digits, so percent * 100 gives basis points.
const FEE_DECIMAL_PRECISION: u128 = 10u128.pow(20);

#[cw_serde]
pub enum Denom {
    Native(String),
    Cw20(cosmwasm_std::Addr),
}

impl Denom {
    /// In Skip a denom is a string: either a native denom or a cw20 address.
    pub fn matches(&self, denom: &str) -> bool {
        match self {
            Denom::Native(n) => n == denom,
            Denom::Cw20(addr) => addr.as_str() == denom,
        }
    }
}

/// NOTE: wasmswap expects PascalCase variants (`Token1`/`Token2`), while
/// `cw_serde` renames everything to snake_case and would emit `token1`.
/// The pool rejects that with: "unknown variant token1, expected Token1
/// or Token2", so the variant names are pinned explicitly.
#[cw_serde]
pub enum TokenSelect {
    #[serde(rename = "Token1")]
    Token1,
    #[serde(rename = "Token2")]
    Token2,
}

#[cw_serde]
pub enum PoolExecuteMsg {
    Swap {
        input_token: TokenSelect,
        input_amount: Uint128,
        min_output: Uint128,
        expiration: Option<Expiration>,
    },
}

#[cw_serde]
pub enum PoolQueryMsg {
    Info {},
    Token1ForToken2Price { token1_amount: Uint128 },
    Token2ForToken1Price { token2_amount: Uint128 },
    Fee {},
}

#[cw_serde]
pub struct InfoResponse {
    pub token1_reserve: Uint128,
    pub token1_denom: Denom,
    pub token2_reserve: Uint128,
    pub token2_denom: Denom,
    pub lp_token_supply: Uint128,
    pub lp_token_address: String,
}

#[cw_serde]
pub struct Token1ForToken2PriceResponse {
    pub token2_amount: Uint128,
}

#[cw_serde]
pub struct Token2ForToken1PriceResponse {
    pub token1_amount: Uint128,
}

#[cw_serde]
pub struct FeeResponse {
    pub owner: Option<String>,
    pub lp_fee_percent: Decimal,
    pub protocol_fee_percent: Decimal,
    pub protocol_fee_recipient: String,
}

impl FeeResponse {
    /// Total pool fee in basis points (2% + 0.4% -> 240).
    pub fn total_fee_bps(&self) -> StdResult<u128> {
        let sum = self
            .lp_fee_percent
            .checked_add(self.protocol_fee_percent)
            .map_err(|e| StdError::generic_err(e.to_string()))?;
        Ok(sum.atomics().u128() / (FEE_DECIMAL_PRECISION / FEE_SCALE_FACTOR))
    }
}

/// Forward pricing: how much comes out for a given input_amount.
/// Mirrors wasmswap's own get_input_price exactly.
pub fn get_input_price(
    input_amount: Uint128,
    input_reserve: Uint128,
    output_reserve: Uint128,
    fee_bps: u128,
) -> StdResult<Uint128> {
    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Err(StdError::generic_err("pool has no liquidity"));
    }
    let fee_reduction = FEE_SCALE_FACTOR
        .checked_sub(fee_bps)
        .ok_or_else(|| StdError::generic_err("fee exceeds 100%"))?;

    let input_with_fee = Uint256::from(input_amount) * Uint256::from(fee_reduction);
    let numerator = input_with_fee * Uint256::from(output_reserve);
    let denominator =
        Uint256::from(input_reserve) * Uint256::from(FEE_SCALE_FACTOR) + input_with_fee;

    Uint128::try_from(numerator / denominator).map_err(|e| StdError::generic_err(e.to_string()))
}

/// Reverse pricing: how much must go in to receive exactly output_amount.
/// wasmswap exposes no reverse price query, so it is derived from reserves and fee.
///
/// From the forward formula:
///   out = in*(S-f)*R_out / (R_in*S + in*(S-f))
/// it follows that:
///   in  = out*R_in*S / ((S-f)*(R_out - out))   plus 1 to round up.
pub fn get_output_price(
    output_amount: Uint128,
    input_reserve: Uint128,
    output_reserve: Uint128,
    fee_bps: u128,
) -> StdResult<Uint128> {
    if input_reserve.is_zero() || output_reserve.is_zero() {
        return Err(StdError::generic_err("pool has no liquidity"));
    }
    if output_amount >= output_reserve {
        return Err(StdError::generic_err(
            "requested output exceeds pool reserve",
        ));
    }
    let fee_reduction = FEE_SCALE_FACTOR
        .checked_sub(fee_bps)
        .ok_or_else(|| StdError::generic_err("fee exceeds 100%"))?;

    let numerator = Uint256::from(output_amount)
        * Uint256::from(input_reserve)
        * Uint256::from(FEE_SCALE_FACTOR);
    let denominator = Uint256::from(fee_reduction)
        * (Uint256::from(output_reserve) - Uint256::from(output_amount));

    let quotient = numerator / denominator;
    // round up, otherwise the input would fall short of the requested output
    let result = if quotient * denominator < numerator {
        quotient + Uint256::one()
    } else {
        quotient
    } + Uint256::one();

    Uint128::try_from(result).map_err(|e| StdError::generic_err(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fee of the live RIP/AXM pool on Axiome: 2% + 0.4%
    const FEE: u128 = 240;

    #[test]
    fn forward_price_matches_pool_formula() {
        // in=1_000_000, R_in=1_000_000_000, R_out=2_000_000_000
        let out = get_input_price(
            Uint128::new(1_000_000),
            Uint128::new(1_000_000_000),
            Uint128::new(2_000_000_000),
            FEE,
        )
        .unwrap();
        // in_with_fee = 1_000_000*9760 = 9_760_000_000
        // num = 9_760_000_000*2_000_000_000; den = 1_000_000_000*10_000 + 9_760_000_000
        let expected = (9_760_000_000u128 * 2_000_000_000u128)
            / (1_000_000_000u128 * 10_000u128 + 9_760_000_000u128);
        assert_eq!(out.u128(), expected);
    }

    #[test]
    fn reverse_price_covers_requested_output() {
        let r_in = Uint128::new(1_000_000_000);
        let r_out = Uint128::new(2_000_000_000);
        for want in [1u128, 1_000, 1_000_000, 123_456_789] {
            let need = get_output_price(Uint128::new(want), r_in, r_out, FEE).unwrap();
            let got = get_input_price(need, r_in, r_out, FEE).unwrap();
            assert!(
                got.u128() >= want,
                "wanted {want}, spent {need}, received {got}"
            );
        }
    }

    #[test]
    fn empty_pool_and_excessive_output_are_rejected() {
        assert!(get_input_price(Uint128::new(1), Uint128::zero(), Uint128::new(1), FEE).is_err());
        assert!(get_output_price(
            Uint128::new(2_000_000_000),
            Uint128::new(1_000_000_000),
            Uint128::new(2_000_000_000),
            FEE
        )
        .is_err());
    }

    #[test]
    fn token_select_serializes_the_way_the_pool_expects() {
        // the pool accepts strictly "Token1"/"Token2"
        assert_eq!(
            serde_json_wasm::to_string(&TokenSelect::Token1).unwrap(),
            "\"Token1\""
        );
        assert_eq!(
            serde_json_wasm::to_string(&TokenSelect::Token2).unwrap(),
            "\"Token2\""
        );
    }

    #[test]
    fn fee_is_converted_to_basis_points() {
        // The live Axiome pool returns exactly these strings:
        // {"lp_fee_percent":"2","protocol_fee_percent":"0.4"} — these are PERCENTS,
        // i.e. Decimal("2") means 2%, not 0.02.
        let fee = FeeResponse {
            owner: None,
            lp_fee_percent: "2".parse().unwrap(),
            protocol_fee_percent: "0.4".parse().unwrap(),
            protocol_fee_recipient: "axm1".to_string(),
        };
        assert_eq!(fee.total_fee_bps().unwrap(), 240);

        // and the degenerate zero-fee case
        let zero = FeeResponse {
            owner: None,
            lp_fee_percent: Decimal::zero(),
            protocol_fee_percent: Decimal::zero(),
            protocol_fee_recipient: "axm1".to_string(),
        };
        assert_eq!(zero.total_fee_bps().unwrap(), 0);
    }
}
