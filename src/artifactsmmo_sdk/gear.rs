use super::{fight_simulator::FightSimulator, items::DamageType, ItemSchemaExt, MonsterSchemaExt};
use artifactsmmo_openapi::models::{ItemSchema, ItemSlot, MonsterSchema, SimpleItemSchema};
use itertools::Itertools;
use std::fmt::Display;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Gear<'a> {
    pub weapon: Option<&'a ItemSchema>,
    pub helmet: Option<&'a ItemSchema>,
    pub shield: Option<&'a ItemSchema>,
    pub body_armor: Option<&'a ItemSchema>,
    pub leg_armor: Option<&'a ItemSchema>,
    pub boots: Option<&'a ItemSchema>,
    pub amulet: Option<&'a ItemSchema>,
    pub ring1: Option<&'a ItemSchema>,
    pub ring2: Option<&'a ItemSchema>,
    pub utility1: Option<&'a ItemSchema>,
    pub utility2: Option<&'a ItemSchema>,
    pub artifact1: Option<&'a ItemSchema>,
    pub artifact2: Option<&'a ItemSchema>,
    pub artifact3: Option<&'a ItemSchema>,
}

impl<'a> Gear<'a> {
    pub fn attack_damage_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                self.weapon.map_or(0.0, |w| {
                    FightSimulator::average_dmg(
                        w.attack_damage(t),
                        self.damage_increase(t),
                        monster.resistance(t),
                    )
                })
            })
            .sum()
    }

    pub fn attack_damage_from(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| FightSimulator::average_dmg(monster.attack_damage(t), 0, self.resistance(t)))
            .sum()
    }

    // TODO: handle consumables
    fn damage_increase(&self, t: DamageType) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.damage_increase(t)))
            .sum()
    }

    pub fn health_increase(&self) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.health()))
            .sum()
    }

    pub fn slot(&self, slot: Slot) -> Option<&ItemSchema> {
        match slot {
            Slot::Weapon => self.weapon,
            Slot::Shield => self.shield,
            Slot::Helmet => self.helmet,
            Slot::BodyArmor => self.body_armor,
            Slot::LegArmor => self.leg_armor,
            Slot::Boots => self.boots,
            Slot::Ring1 => self.ring1,
            Slot::Ring2 => self.ring2,
            Slot::Amulet => self.amulet,
            Slot::Artifact1 => self.artifact1,
            Slot::Artifact2 => self.artifact2,
            Slot::Artifact3 => self.artifact3,
            Slot::Utility1 => self.utility1,
            Slot::Utility2 => self.utility2,
        }
    }

    fn resistance(&self, t: DamageType) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.resistance(t)))
            .sum()
    }

    pub fn haste(&self) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.haste()))
            .sum()
    }
}

impl Display for Gear<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Weapon: {:?}", self.weapon.map(|w| w.name.to_string()))?;
        writeln!(f, "Shield: {:?}", self.shield.map(|w| w.name.to_string()))?;
        writeln!(f, "Helmet: {:?}", self.helmet.map(|w| w.name.to_string()))?;
        writeln!(
            f,
            "Body Armor: {:?}",
            self.body_armor.map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Leg Armor: {:?}",
            self.leg_armor.map(|w| w.name.to_string())
        )?;
        writeln!(f, "Boots: {:?}", self.boots.map(|w| w.name.to_string()))?;
        writeln!(f, "Ring 1: {:?}", self.ring1.map(|w| w.name.to_string()))?;
        writeln!(f, "Ring 2: {:?}", self.ring2.map(|w| w.name.to_string()))?;
        writeln!(f, "Amulet: {:?}", self.amulet.map(|w| w.name.to_string()))?;
        writeln!(
            f,
            "Artifact 1: {:?}",
            self.artifact1.map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Artifact 2: {:?}",
            self.artifact2.map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Artifact 3: {:?}",
            self.artifact3.map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Consumable 1: {:?}",
            self.utility1.map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Consumable 2: {:?}",
            self.utility2.map(|w| w.name.to_string())
        )
    }
}

impl From<Gear<'_>> for Vec<SimpleItemSchema> {
    fn from(val: Gear<'_>) -> Self {
        let mut i = Slot::iter()
            .filter_map(|s| {
                if s.is_ring_1() || s.is_ring_2() {
                    if let Some(item) = val.slot(s) {
                        return Some(SimpleItemSchema {
                            code: item.code.to_owned(),
                            quantity: if s.is_utility_1() || s.is_utility_2() {
                                100
                            } else {
                                1
                            },
                        });
                    }
                }
                None
            })
            .collect_vec();
        match (val.ring1, val.ring2) {
            (Some(r1), Some(r2)) => {
                if r1 == r2 {
                    i.push(SimpleItemSchema {
                        code: r1.code.to_owned(),
                        quantity: 2,
                    })
                } else {
                    i.push(SimpleItemSchema {
                        code: r1.code.to_owned(),
                        quantity: 1,
                    });
                    i.push(SimpleItemSchema {
                        code: r2.code.to_owned(),
                        quantity: 1,
                    });
                }
            }
            (Some(r), None) => i.push(SimpleItemSchema {
                code: r.code.to_owned(),
                quantity: 1,
            }),
            (None, Some(r)) => i.push(SimpleItemSchema {
                code: r.code.to_owned(),
                quantity: 1,
            }),
            (None, None) => (),
        }
        i
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Display, AsRefStr, EnumString, EnumIter, EnumIs,
)]
#[strum(serialize_all = "snake_case")]
pub enum Slot {
    #[default]
    Weapon,
    Shield,
    Helmet,
    BodyArmor,
    LegArmor,
    Boots,
    Ring1,
    Ring2,
    Amulet,
    Artifact1,
    Artifact2,
    Artifact3,
    Utility1,
    Utility2,
}

impl Slot {
    pub fn max_quantity(&self) -> i32 {
        match self {
            Slot::Weapon
            | Slot::Shield
            | Slot::Helmet
            | Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Boots
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet
            | Slot::Artifact1
            | Slot::Artifact2
            | Slot::Artifact3 => 1,
            Slot::Utility1 => 100,
            Slot::Utility2 => 100,
        }
    }
}

impl From<ItemSlot> for Slot {
    fn from(value: ItemSlot) -> Self {
        match value {
            ItemSlot::Weapon => Self::Weapon,
            ItemSlot::Shield => Self::Shield,
            ItemSlot::Helmet => Self::Helmet,
            ItemSlot::BodyArmor => Self::BodyArmor,
            ItemSlot::LegArmor => Self::LegArmor,
            ItemSlot::Boots => Self::Boots,
            ItemSlot::Ring1 => Self::Ring1,
            ItemSlot::Ring2 => Self::Ring2,
            ItemSlot::Amulet => Self::Amulet,
            ItemSlot::Artifact1 => Self::Artifact1,
            ItemSlot::Artifact2 => Self::Artifact2,
            ItemSlot::Artifact3 => Self::Artifact3,
            ItemSlot::Utility1 => Self::Utility1,
            ItemSlot::Utility2 => Self::Utility2,
        }
    }
}

impl From<Slot> for ItemSlot {
    fn from(value: Slot) -> Self {
        match value {
            Slot::Weapon => Self::Weapon,
            Slot::Shield => Self::Shield,
            Slot::Helmet => Self::Helmet,
            Slot::BodyArmor => Self::BodyArmor,
            Slot::LegArmor => Self::LegArmor,
            Slot::Boots => Self::Boots,
            Slot::Ring1 => Self::Ring1,
            Slot::Ring2 => Self::Ring2,
            Slot::Amulet => Self::Amulet,
            Slot::Artifact1 => Self::Artifact1,
            Slot::Artifact2 => Self::Artifact2,
            Slot::Artifact3 => Self::Artifact3,
            Slot::Utility1 => Self::Utility1,
            Slot::Utility2 => Self::Utility2,
        }
    }
}
