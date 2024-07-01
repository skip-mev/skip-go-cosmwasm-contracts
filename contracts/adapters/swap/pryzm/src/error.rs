use cosmwasm_std::StdError;
use skip::error::SkipError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

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

    #[error("provided pool string is not a valid swap route")]
    InvalidPool,

    #[error("swap_operations cannot be empty")]
    SwapOperationsEmpty,

    #[error("coin_in denom must match the first swap operation's denom in")]
    CoinInDenomMismatch,

    #[error("coin_out denom must match the last swap operation's denom out")]
    CoinOutDenomMismatch,

    #[error("Asset Must Be Native, Pryzm Does Not Support CW20 Tokens")]
    AssetNotNative,

    #[error("SubMsgResponse does not contain data")]
    MissingResponseData,
}
