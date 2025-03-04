use cosmwasm_std::StdError;
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Decode(#[from] prost::DecodeError),

    #[error("SubMsgResponse does not contain data")]
    MissingResponseData,

    #[error("Failed to receive ibc funds to refund the user")]
    NoFundsToRefund,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("ACK ID already exists for channel ID {channel_id} and sequence ID {sequence_id}")]
    AckIDAlreadyExists {
        channel_id: String,
        sequence_id: u64,
    },

    #[error("Failed to decode packet data into fungible token packet data")]
    FailedToDecodePacketData,
}
