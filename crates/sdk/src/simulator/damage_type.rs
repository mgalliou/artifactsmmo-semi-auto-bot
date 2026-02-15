use strum_macros::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, Copy, Clone, PartialEq, AsRefStr, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DamageType {
    Fire,
    Earth,
    Water,
    Air,
}

impl DamageType {
    pub fn into_attack(&self) -> &'static str {
        match self {
            DamageType::Fire => "attack_fire",
            DamageType::Earth => "attack_earth",
            DamageType::Water => "attack_water",
            DamageType::Air => "attack_air",
        }
    }

    pub fn into_dmg(&self) -> &'static str {
        match self {
            DamageType::Fire => "dmg_fire",
            DamageType::Earth => "dmg_earth",
            DamageType::Water => "dmg_water",
            DamageType::Air => "dmg_air",
        }
    }

    pub fn into_boost_dmg(&self) -> &'static str {
        match self {
            DamageType::Fire => "boost_dmg_fire",
            DamageType::Earth => "boost_dmg_earth",
            DamageType::Water => "boost_dmg_water",
            DamageType::Air => "boost_dmg_air",
        }
    }

    pub fn into_res(&self) -> &'static str {
        match self {
            DamageType::Fire => "res_fire",
            DamageType::Earth => "res_earth",
            DamageType::Water => "res_water",
            DamageType::Air => "res_air",
        }
    }
}
