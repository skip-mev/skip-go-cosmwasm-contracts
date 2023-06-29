use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Payment(#[from] cw_utils::PaymentError),

    #[error("swap_operations cannot be empty")]
    SwapOperationsEmpty,

    #[error("coin_in denom must match the first swap operation's denom in")]
    CoinInDenomMismatch,

    #[error("coin_out denom must match the last swap operation's denom out")]
    CoinOutDenomMismatch,
}
