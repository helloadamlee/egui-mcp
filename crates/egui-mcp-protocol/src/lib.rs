use thiserror::Error;

pub mod messages;
pub mod types;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid message type: {0}")]
    InvalidMessage(String),

    #[error("Message handling error: {0}")]
    Handling(String),
}

pub type ProtocolResult<T> = Result<T, ProtocolError>;
