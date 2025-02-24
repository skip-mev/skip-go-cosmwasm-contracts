use crate::ibc::IbcLifecycleComplete;

use cosmwasm_schema::cw_serde;

// SudoType used to give info in response attributes when the sudo function is called
pub enum SudoType {
    Response,
    Error,
    Timeout,
}

// Implement the From trait for SudoType to convert it to a string to be used in response attributes
impl From<SudoType> for String {
    fn from(sudo_type: SudoType) -> Self {
        match sudo_type {
            SudoType::Response => "sudo_ack_success".into(),
            SudoType::Error => "sudo_ack_error_and_bank_send".into(),
            SudoType::Timeout => "sudo_timeout_and_bank_send".into(),
        }
    }
}

// Message type for Osmosis `sudo` entry_point to interact with callbacks from the ibc hooks module
#[cw_serde]
pub enum OsmosisSudoMsg {
    IbcLifecycleComplete(IbcLifecycleComplete),
}
