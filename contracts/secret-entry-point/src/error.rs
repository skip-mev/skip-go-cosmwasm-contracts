use cosmwasm_std::{Addr, OverflowError, StdError};
use secret_skip::error::SkipError;
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

    #[error("Duplicate Swap Venue Name Provided")]
    DuplicateSwapVenueName,

    #[error("IBC fee denom differs from asset received without a fee swap to convert")]
    IBCFeeDenomDiffersFromAssetReceived,

    ////////////////
    /// FEE SWAP ///
    ////////////////

    #[error("Fee Swap Not Allowed: No IBC Fees Provided")]
    FeeSwapWithoutIbcFees,

    #[error("Fee Swap Asset In Denom Differs From Asset Sent To Contract")]
    FeeSwapAssetInDenomMismatch,

    /////////////////
    /// USER SWAP ///
    /////////////////

    #[error("User Swap Asset In Denom Differs From Asset Sent To Contract")]
    UserSwapAssetInDenomMismatch,

    #[error("No Refund Address Provided For Swap Exact Asset Out User Swap")]
    NoRefundAddress,

    ////////////////////////
    /// POST SWAP ACTION ///
    ////////////////////////

    #[error("Received Less Asset From Swaps Than Minimum Asset Required")]
    ReceivedLessAssetFromSwapsThanMinAsset,

    #[error("Contract Call Address Cannot Be The Entry Point Or Adapter Contracts")]
    ContractCallAddressBlocked,

    #[error(
        "IBC Transfer Adapter Only Supports Native Coins, Cw20 IBC Transfers Are Contract Calls"
    )]
    NonNativeIbcTransfer,

    #[error("Hyperlane Transfer Adapter Only Supports Native Coins")]
    NonNativeHplTransfer,

    #[error("Reply id: {0} not valid")]
    ReplyIdError(u64),

    //////////////////
    ///   ACTION   ///
    //////////////////

    #[error("No Minimum Asset Provided with Exact Out Action")]
    NoMinAssetProvided,

    #[error("Sent Asset and Min Asset Denoms Do Not Match with Exact Out Action")]
    ActionDenomMismatch,

    #[error("Remaining Asset Less Than Min Asset with Exact Out Action")]
    RemainingAssetLessThanMinAsset,

    #[error("No Snip20 Receive Msg Provided")]
    NoSnip20ReceiveMsg,

    #[error("Native Coin Not Supported")]
    NativeCoinNotSupported,

    #[error("Invalid Snip20 Sender")]
    InvalidSnip20Sender,

    #[error("Snip20 Token Not Registered {0}")]
    TokenNotRegistered(Addr),
}
