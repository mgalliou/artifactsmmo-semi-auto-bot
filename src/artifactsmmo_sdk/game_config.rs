use super::char_config::CharConfig;
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct GameConfig {
    pub base_url: String,
    pub token: String,
    pub characters: Vec<CharConfig>,
}
