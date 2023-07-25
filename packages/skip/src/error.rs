use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum SkipError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Ibc Fees Are Not A Single Coin, Either Multiple Denoms Or No Coin Specified")]
    IbcFeesNotOneCoin,
}
