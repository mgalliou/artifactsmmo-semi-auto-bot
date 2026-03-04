use strum_macros::{AsRefStr, EnumIter, EnumString};

#[derive(Debug, Copy, Clone, PartialEq, Eq, AsRefStr, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DamageType {
    Fire,
    Earth,
    Water,
    Air,
}

impl DamageType {
    pub const fn into_attack(&self) -> &'static str {
        match self {
            Self::Fire => "attack_fire",
            Self::Earth => "attack_earth",
            Self::Water => "attack_water",
            Self::Air => "attack_air",
        }
    }

    pub const fn into_dmg(&self) -> &'static str {
        match self {
            Self::Fire => "dmg_fire",
            Self::Earth => "dmg_earth",
            Self::Water => "dmg_water",
            Self::Air => "dmg_air",
        }
    }

    pub const fn into_boost_dmg(&self) -> &'static str {
        match self {
            Self::Fire => "boost_dmg_fire",
            Self::Earth => "boost_dmg_earth",
            Self::Water => "boost_dmg_water",
            Self::Air => "boost_dmg_air",
        }
    }

    pub const fn into_res(&self) -> &'static str {
        match self {
            Self::Fire => "res_fire",
            Self::Earth => "res_earth",
            Self::Water => "res_water",
            Self::Air => "res_air",
        }
    }
}
