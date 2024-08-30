use super::character::Role;

#[derive(Default)]
pub struct CharConfig {
    pub role: Role,
    pub fight: bool,
    pub fight_target: Option<String>,
    pub level: bool,
    pub mine: bool,
    pub mine_craft: bool,
    pub mine_resource: Option<String>,
    pub lumber: bool,
    pub lumber_craft: bool,
    pub lumber_resource: Option<String>,
    pub fish: bool,
    pub fish_resource: Option<String>,
    pub cook: bool,
    pub level_cook: bool,
    pub weaponcraft: bool,
    pub level_weaponcraft: bool,
    pub gearcraft: bool,
    pub level_gearcraft: bool,
    pub jewelcraft: bool,
    pub level_jewelcraft: bool,
}
