//! Error types for MoFA Dora Bridge

use thiserror::Error;

/// Result type alias for bridge operations
pub type BridgeResult<T> = Result<T, BridgeError>;

/// Errors that can occur in bridge operations
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Failed to connect to Dora: {0}")]
    ConnectionFailed(String),

    #[error("Bridge already connected")]
    AlreadyConnected,

    #[error("Bridge not connected")]
    NotConnected,

    #[error("Failed to send data: {0}")]
    SendFailed(String),

    #[error("Failed to receive data: {0}")]
    ReceiveFailed(String),

    #[error("Invalid data format: {0}")]
    InvalidData(String),

    #[error("Dataflow not found: {0}")]
    DataflowNotFound(String),

    #[error("Failed to parse dataflow: {0}")]
    ParseError(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Dataflow already running")]
    DataflowAlreadyRunning,

    #[error("Dataflow not running")]
    DataflowNotRunning,

    #[error("Failed to start dataflow: {0}")]
    StartFailed(String),

    #[error("Failed to stop dataflow: {0}")]
    StopFailed(String),

    #[error("Audio device error: {0}")]
    AudioError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Channel send error")]
    ChannelSendError,

    #[error("Channel receive error")]
    ChannelReceiveError,

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Thread spawn failed: {0}")]
    ThreadSpawnFailed(String),

    #[error("Thread join failed")]
    ThreadJoinFailed,

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Bridge already running")]
    AlreadyRunning,

    #[error("Unknown error: {0}")]
    Unknown(String),
}
