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

    #[error(transparent)]
    CheckedFromRatioError(#[from] cosmwasm_std::CheckedFromRatioError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("provided pool string is not a valid swap route: {msg:?}")]
    InvalidPool { msg: String },

    #[error("swap_operations cannot be empty")]
    SwapOperationsEmpty,

    #[error("coin_in denom must match the first swap operation's denom in")]
    CoinInDenomMismatch,

    #[error("coin_out denom must match the last swap operation's denom out")]
    CoinOutDenomMismatch,

    #[error("Asset Must Be Native, Pryzm Does Not Support CW20 Tokens")]
    AssetNotNative,

    #[error("Unexpected message response received: {msg:?}")]
    InvalidMsgResponse { msg: String },

    #[error("Unexpected query response received: {msg:?}")]
    InvalidQueryResponse { msg: String },

    #[error("InvalidState: {msg:?}")]
    InvalidState { msg: String },
}
