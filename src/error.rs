use jsonrpsee::{
  client::RequestError,
  transport::ws::WsNewDnsError,
};
use sp_core::crypto::{ PublicError };

pub type Result<T> = std::result::Result<T, Error>;

/// Error enum.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Io error.
  #[error("Io error: {0}")]
  Io(#[from] std::io::Error),
  /// Codec error.
  #[error("Scale codec error: {0}")]
  Codec(#[from] codec::Error),
  /// Rpc error.
  #[error("Rpc error: {0}")]
  Rpc(#[from] RequestError),
  /// Error that can happen during the initial websocket handshake
  #[error("Rpc error: {0}")]
  WsHandshake(#[from] WsNewDnsError),
  /// Serde serialization error
  #[error("Serde json error: {0}")]
  Serialization(#[from] serde_json::error::Error),

  #[error("Deserialize toml error: {0}")]
  DeserializeToml(#[from] toml::de::Error),

  #[error("Serialize toml error: {0}")]
  SerializeToml(#[from] toml::ser::Error),

  #[error("Invalid SS58 address")]
  PublicKey(PublicError),

  #[error("Websocket Error: {0}")]
  WsError(#[from] tungstenite::Error),

  /// Other error.
  #[error("Other error: {0}")]
  Other(String),
}

impl From<PublicError> for Error {
  fn from(error: PublicError) -> Self {
    Error::PublicKey(error)
  }
}

impl From<&str> for Error {
  fn from(error: &str) -> Self {
    Error::Other(error.into())
  }
}

impl From<String> for Error {
  fn from(error: String) -> Self {
    Error::Other(error)
  }
}