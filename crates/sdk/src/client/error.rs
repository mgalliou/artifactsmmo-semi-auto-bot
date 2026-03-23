use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("api error: {0}")]
    Api(#[from] Box<dyn StdError + Send + Sync>),
}
