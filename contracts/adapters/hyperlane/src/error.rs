use cosmwasm_std::StdError;
use skip::error::SkipError;
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Skip(#[from] SkipError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error(transparent)]
    Payment(#[from] cw_utils::PaymentError),
}
