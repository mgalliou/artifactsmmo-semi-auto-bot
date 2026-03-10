use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("API Error: {0}")]
    Api(#[from] Box<dyn StdError + Send + Sync>),
}
