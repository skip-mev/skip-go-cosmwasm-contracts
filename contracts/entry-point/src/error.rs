use cosmwasm_std::{OverflowError, StdError};
use skip::error::SkipError;
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    ///////////////
    /// GENERAL ///
    ///////////////

    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Skip(#[from] SkipError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Payment(#[from] cw_utils::PaymentError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Timeout Timestamp Less Than Current Timestamp")]
    Timeout,

    #[error("IBC fee denom differs from coin received without a fee swap to convert")]
    IBCFeeDenomDiffersFromCoinReceived,

    ////////////////
    /// FEE SWAP ///
    ////////////////

    #[error("Fee Swap Not Allowed: No IBC Fees Provided")]
    FeeSwapWithoutIbcFees,

    #[error("Fee Swap Coin In Denom Differs From Coin Sent To Contract")]
    FeeSwapCoinInDenomMismatch,

    /////////////////
    /// USER SWAP ///
    /////////////////

    #[error("User Swap Coin In Denom Differs From Coin Sent To Contract")]
    UserSwapCoinInDenomMismatch,

    #[error("No Refund Address Provided For Swap Exact Coin Out User Swap")]
    NoRefundAddress,

    ////////////////////////
    /// POST SWAP ACTION ///
    ////////////////////////

    #[error("Received Less Coin From Swaps Than Minimum Coin Required")]
    ReceivedLessCoinFromSwapsThanMinCoin,

    #[error("Contract Call Address Cannot Be The Entry Point Or Adapter Contracts")]
    ContractCallAddressBlocked,
}
