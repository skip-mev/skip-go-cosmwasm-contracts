use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SkipError {
    // GENERAL
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    // SWAP OPERATIONS
    #[error("Swap Operations Empty")]
    SwapOperationsEmpty,

    #[error("First Swap Operations' Denom In Differs From Swap Coin In Denom")]
    SwapOperationsCoinInDenomMismatch,

    #[error("Last Swap Operations' Denom Out Differs From Swap Coin Out Denom")]
    SwapOperationsCoinOutDenomMismatch,
}
