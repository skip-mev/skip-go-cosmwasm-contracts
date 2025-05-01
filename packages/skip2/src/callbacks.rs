// SourceCallbackType used to give info in response attributes when the source callback function is called
pub enum SourceCallbackType {
    Response,
    Error,
    Timeout,
}

// Implement the From trait for SourceCallbackType to convert it to a string to be used in response attributes
impl From<SourceCallbackType> for String {
    fn from(source_callback_type: SourceCallbackType) -> Self {
        match source_callback_type {
            SourceCallbackType::Response => "source_callback_ack_success".into(),
            SourceCallbackType::Error => "source_callback_ack_error_and_bank_send".into(),
            SourceCallbackType::Timeout => "source_callback_timeout_and_bank_send".into(),
        }
    }
}
