#[derive(Debug)]
pub enum HttpXError {
    Transport(std::io::Error),
    ProtocolViolation(String),
    IntentMismatch,
    CreditExhausted,
    CodecError(String),
}

impl From<std::io::Error> for HttpXError {
    fn from(e: std::io::Error) -> Self {
        HttpXError::Transport(e)
    }
}
