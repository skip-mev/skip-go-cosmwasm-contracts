use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Decode(#[from] prost::DecodeError),

    #[error(transparent)]
    JsonDecode(#[from] serde_json_wasm::de::Error),

    #[error(transparent)]
    JsonEncode(#[from] serde_json_wasm::ser::Error),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error("IBC fees are not supported, vectors must be empty")]
    IbcFeesNotSupported,

    #[error("SubMsgResponse does not contain data")]
    MissingResponseData,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("ACK ID already exists for channel ID {channel_id} and sequence ID {sequence_id}")]
    AckIDAlreadyExists {
        channel_id: String,
        sequence_id: u64,
    },
}
