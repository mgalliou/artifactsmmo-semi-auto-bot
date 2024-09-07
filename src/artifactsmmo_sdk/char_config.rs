use super::character::Role;
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CharConfig {
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub fight_target: Option<String>,
    #[serde(default)]
    pub do_tasks: bool,
    #[serde(default)]
    pub level: bool,
    #[serde(default)]
    pub target_item: Option<String>,
    #[serde(default)]
    pub process_gathered: bool,
    #[serde(default)]
    pub cook: bool,
    #[serde(default)]
    pub level_cook: bool,
    #[serde(default)]
    pub weaponcraft: bool,
    #[serde(default)]
    pub level_weaponcraft: bool,
    #[serde(default)]
    pub gearcraft: bool,
    #[serde(default)]
    pub level_gearcraft: bool,
    #[serde(default)]
    pub jewelcraft: bool,
    #[serde(default)]
    pub level_jewelcraft: bool,
    // process gathered resource
    #[serde(default)]
    pub craft: bool,
    // process target resource from bank
    #[serde(default)]
    pub craft_from_bank: bool,
}
