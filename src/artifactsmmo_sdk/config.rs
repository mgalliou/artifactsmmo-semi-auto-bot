use super::char_config::CharConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub token: String,
    pub characters: Vec<CharConfig>,
}
