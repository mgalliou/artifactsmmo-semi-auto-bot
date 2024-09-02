use super::character::Role;

#[derive(Default)]
pub struct CharConfig {
    pub role: Role,
    pub fight_target: Option<String>,
    pub level: bool,
    pub resource: Option<String>,
    pub process_gathered: bool,
    pub cook: bool,
    pub level_cook: bool,
    pub weaponcraft: bool,
    pub level_weaponcraft: bool,
    pub gearcraft: bool,
    pub level_gearcraft: bool,
    pub jewelcraft: bool,
    pub level_jewelcraft: bool,
    // process gathered resource
    pub craft: bool,
    // process target resource from bank
    pub craft_from_bank: bool
}
