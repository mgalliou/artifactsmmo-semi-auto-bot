use std::sync::RwLock;

use super::char_config::CharConfig;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct GameConfig {
    pub base_url: String,
    pub token: String,
    pub characters: Vec<RwLock<CharConfig>>,
}

impl GameConfig {
    pub fn from_file() -> Self {
        Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap()
    }
}
