use serde::Deserialize;
use super::char_config::CharConfig;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub token: String,
    pub characters: Vec<CharConfig>,
}
