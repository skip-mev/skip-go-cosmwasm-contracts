use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use skip::error::SkipError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Skip(#[from] SkipError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Overflow(#[from] cosmwasm_std::OverflowError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("swap_operations cannot be empty")]
    SwapOperationsEmpty,

    #[error("Asset Must Be Native, there is no Support for CW20 Tokens")]
    AssetNotNative,

    #[error("coin_out denom must match the last swap operation's denom out")]
    CoinOutDenomMismatch,
}
