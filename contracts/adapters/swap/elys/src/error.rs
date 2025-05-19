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

    #[error("Asset Must Be Native, Elys Does Not Support CW20 Tokens")]
    AssetNotNative,

    #[error("TokenOut Not Found in swap exact asset in estimation")]
    TokenOutNotFound,

    #[error("TokenIn Not Found in swap exact asset out estimation")]
    TokenInNotFound,

    #[error("SpotPrice Not Found")]
    SpotPriceNotFound,
}
