use cosmwasm_std::StdError;
use skip::error::SkipError;
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Skip(#[from] SkipError),

    #[error(transparent)]
    Payment(#[from] cw_utils::PaymentError),

    #[error(transparent)]
    Overflow(#[from] cosmwasm_std::OverflowError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Parse Int error raised: invalid pool String to pool id u64 conversion")]
    ParseIntPoolID(#[from] std::num::ParseIntError),

    #[error("swap_operations cannot be empty")]
    SwapOperationsEmpty,

    #[error("coin_in denom must match the first swap operation's denom in")]
    CoinInDenomMismatch,

    #[error("coin_out denom must match the last swap operation's denom out")]
    CoinOutDenomMismatch,

    #[error("Asset Must Be Native, Osmosis Does Not Support CW20 Tokens")]
    AssetNotNative,

    #[error("Swap operation denom mismatch. tokenOut of previous swap must be tokenIn of next swap")]
    SwapOperationDenomMismatch,

    #[error("failed to convert uint to int. value of coin amount as Uint128 exceeds max possible Int128 amount")]
    ConversionError,

    #[error("swap operation denom-in and denom-out are the same.")]
    SameSwapDenoms,

    #[error("Route must me length 1. Smart Swap is not supported")]
    SmartSwapUnsupported,

    #[error("Simulation Error. Unexpected output denom")]
    SmartSwapUnexpectedOut,

    #[error("Simulation Error. Not Enough Liquidity")]
    NoLiquidityToParse,

    #[error("Unsupported Price. Price is too small, truncating either causes zero price or too large price discrepancy")]
    PriceTruncateError,
}
