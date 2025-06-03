use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents an error returned in a JSON-RPC response from Deribit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug)]
pub enum Error {
    Api(ApiError),
    Json(serde_json::Error),
    WebSocket(tungstenite::Error),
    Channel(tokio::sync::oneshot::error::RecvError),
    Io(std::io::Error),
    Logic(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Api(err) => write!(f, "API error: {}", err.message),
            Error::Json(err) => write!(f, "JSON error: {}", err),
            Error::WebSocket(err) => write!(f, "WebSocket error: {}", err),
            Error::Channel(err) => write!(f, "Channel error: {}", err),
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Logic(msg) => write!(f, "Logic error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Api(_) => None,
            Error::Json(err) => Some(err),
            Error::WebSocket(err) => Some(err),
            Error::Channel(err) => Some(err),
            Error::Io(err) => Some(err),
            Error::Logic(_) => None,
        }
    }
}

impl From<ApiError> for Error {
    fn from(err: ApiError) -> Self {
        Error::Api(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<tungstenite::Error> for Error {
    fn from(err: tungstenite::Error) -> Self {
        Error::WebSocket(err)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Self {
        Error::Channel(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
} 