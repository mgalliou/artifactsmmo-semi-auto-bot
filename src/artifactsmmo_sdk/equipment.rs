use super::{
    compute_damage,
    items::{DamageType, Slot},
    ItemSchemaExt, MonsterSchemaExt,
};
use artifactsmmo_openapi::models::{ItemSchema, MonsterSchema};
use strum::IntoEnumIterator;

#[derive(Default, Debug, Clone, Copy)]
pub struct Equipment<'a> {
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

impl<'a> Equipment<'a> {
    pub fn attack_damage_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                self.weapon.map_or(0.0, |w| {
                    compute_damage(
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
            .map(|t| compute_damage(monster.attack_damage(t), 0, self.resistance(t)))
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
