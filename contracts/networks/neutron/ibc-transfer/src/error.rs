use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Error decoding Sub Msg Response data to MsgTransferResponse")]
    Decode(#[from] prost::DecodeError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("SubMsgResponse does not contain data")]
    MissingResponseData,

    #[error("Channel ID not found")]
    ChannelIDNotFound,

    #[error("Sequence not found")]
    SequenceNotFound,

    #[error("Failed to receive ibc funds to refund the user")]
    NoFundsToRefund,

    #[error("ACK ID already exists for channel ID {channel_id} and sequence ID {sequence_id}")]
    AckIDAlreadyExists {
        channel_id: String,
        sequence_id: u64,
    },
}
