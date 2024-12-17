use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SkipError {
    ///////////////
    /// GENERAL ///
    ///////////////

    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error(transparent)]
    Payment(#[from] cw_utils::PaymentError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    ////////////
    /// SWAP ///
    ////////////

    #[error("Swap Operations Empty")]
    SwapOperationsEmpty,

    #[error("First Swap Operations' Denom In Differs From Swap Asset In Denom")]
    SwapOperationsAssetInDenomMismatch,

    #[error("Last Swap Operations' Denom Out Differs From Swap Asset Out Denom")]
    SwapOperationsAssetOutDenomMismatch,

    #[error("Routes Empty")]
    RoutesEmpty,

    ///////////
    /// IBC ///
    ///////////

    #[error("Ibc Fees Are Not A Single Coin, Either Multiple Denoms Or No Coin Specified")]
    IbcFeesNotOneCoin,

    /////////////
    /// ASSET ///
    /////////////

    #[error("Native Coin Sent To Contract Does Not Match Asset")]
    InvalidNativeCoin,

    #[error("Cw20 Coin Sent To Contract Does Not Match Asset")]
    InvalidCw20Coin,
}
