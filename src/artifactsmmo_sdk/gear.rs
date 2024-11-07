use super::{average_dmg, items::DamageType, ItemSchemaExt, MonsterSchemaExt};
use artifactsmmo_openapi::models::{
    equip_schema, unequip_schema, ItemSchema, MonsterSchema, SimpleItemSchema,
};
use itertools::Itertools;
use std::fmt::Display;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Gear<'a> {
    pub weapon: Option<&'a ItemSchema>,
    pub shield: Option<&'a ItemSchema>,
    pub helmet: Option<&'a ItemSchema>,
    pub body_armor: Option<&'a ItemSchema>,
    pub leg_armor: Option<&'a ItemSchema>,
    pub boots: Option<&'a ItemSchema>,
    pub ring1: Option<&'a ItemSchema>,
    pub ring2: Option<&'a ItemSchema>,
    pub amulet: Option<&'a ItemSchema>,
    pub artifact1: Option<&'a ItemSchema>,
    pub artifact2: Option<&'a ItemSchema>,
    pub artifact3: Option<&'a ItemSchema>,
    pub consumable1: Option<&'a ItemSchema>,
    pub consumable2: Option<&'a ItemSchema>,
}

impl<'a> Gear<'a> {
    pub fn attack_damage_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                self.weapon.map_or(0.0, |w| {
                    average_dmg(
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
            .map(|t| average_dmg(monster.attack_damage(t), 0, self.resistance(t)))
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
            Slot::Consumable1 => self.consumable1,
            Slot::Consumable2 => self.consumable2,
        }
    }

    fn resistance(&self, t: DamageType) -> i32 {
        Slot::iter()
            .map(|s| self.slot(s).map_or(0, |i| i.resistance(t)))
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
            self.consumable1.map(|w| w.name.to_string())
        )?;
        writeln!(
            f,
            "Consumable 2: {:?}",
            self.consumable2.map(|w| w.name.to_string())
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
                            quantity: if s.is_consumable_1() || s.is_consumable_2() {
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

#[derive(Debug, Default, Copy, Clone, PartialEq, Display, AsRefStr, EnumString, EnumIter, EnumIs)]
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
    Consumable1,
    Consumable2,
}

impl From<equip_schema::Slot> for Slot {
    fn from(value: equip_schema::Slot) -> Self {
        match value {
            equip_schema::Slot::Weapon => Self::Weapon,
            equip_schema::Slot::Shield => Self::Shield,
            equip_schema::Slot::Helmet => Self::Helmet,
            equip_schema::Slot::BodyArmor => Self::BodyArmor,
            equip_schema::Slot::LegArmor => Self::LegArmor,
            equip_schema::Slot::Boots => Self::Boots,
            equip_schema::Slot::Ring1 => Self::Ring1,
            equip_schema::Slot::Ring2 => Self::Ring2,
            equip_schema::Slot::Amulet => Self::Amulet,
            equip_schema::Slot::Artifact1 => Self::Artifact1,
            equip_schema::Slot::Artifact2 => Self::Artifact2,
            equip_schema::Slot::Artifact3 => Self::Artifact3,
            equip_schema::Slot::Consumable1 => Self::Consumable1,
            equip_schema::Slot::Consumable2 => Self::Consumable2,
        }
    }
}

impl From<Slot> for equip_schema::Slot {
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
            Slot::Consumable1 => Self::Consumable1,
            Slot::Consumable2 => Self::Consumable2,
        }
    }
}

impl From<Slot> for unequip_schema::Slot {
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
            Slot::Consumable1 => Self::Consumable1,
            Slot::Consumable2 => Self::Consumable2,
        }
    }
}
