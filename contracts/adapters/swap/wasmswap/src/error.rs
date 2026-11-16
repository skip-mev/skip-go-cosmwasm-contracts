use cosmwasm_std::{OverflowError, StdError};
use skip::error::SkipError;
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Skip(#[from] SkipError),

    #[error(transparent)]
    Payment(#[from] cw_utils::PaymentError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("swap_operations cannot be empty")]
    SwapOperationsEmpty,

    #[error("Contract has no balance of offer asset")]
    NoOfferAssetAmount,

    #[error("denom_in is not one of the pool's two tokens")]
    PoolDenomMismatch,

    #[error("pool has no liquidity for the requested direction")]
    PoolEmpty,

    #[error("requested asset out exceeds the pool's reserve")]
    AssetOutExceedsReserve,
}
