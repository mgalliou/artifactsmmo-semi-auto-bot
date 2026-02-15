use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("API Error: {0}")]
    Api(Box<dyn StdError + Send + Sync>),
}
