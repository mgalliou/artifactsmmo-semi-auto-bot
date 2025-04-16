use crate::{gear::Gear, items::ItemSchemaExt};
use artifactsmmo_openapi::models::{FightResult, MonsterSchema};
use std::cmp::max;

const BASE_HP: i32 = 115;
const MAX_TURN: i32 = 100;
const HP_PER_LEVEL: i32 = 5;

pub struct Simulator {}

impl Simulator {
    /// Compute the average damage an attack will do against the given `target_resistance`. Block
    /// chance is considered as a global damage reduction (30 resistence reduce the computed damage by
    /// 3%).
    pub fn average_dmg(attack_damage: i32, damage_increase: i32, target_resistance: i32) -> f32 {
        let mut dmg = attack_damage as f32 + (attack_damage as f32 * damage_increase as f32 * 0.01);
        dmg -= dmg * target_resistance as f32 * 0.01;
        // TODO: include this in a different function and rename this one
        //if target_resistance > 0 {
        //    dmg *= 1.0 - (target_resistance as f32 / 1000.0)
        //};
        dmg
    }

    pub fn fight(
        level: i32,
        missing_hp: i32,
        gear: &Gear,
        monster: &MonsterSchema,
        ignore_death: bool,
    ) -> Fight {
        let base_hp = BASE_HP + HP_PER_LEVEL * level;
        let starting_hp = base_hp + gear.health_increase() - missing_hp;
        let mut hp = starting_hp;
        let mut monster_hp = monster.hp;
        let mut turns = 1;

        loop {
            if turns % 2 == 1 {
                monster_hp -= gear.attack_damage_against(monster);
                if monster_hp <= 0 {
                    break;
                }
            } else {
                if hp < (base_hp + gear.health_increase()) / 2 {
                    hp += gear.utility1.as_ref().map(|u| u.restore()).unwrap_or(0);
                    hp += gear.utility2.as_ref().map(|u| u.restore()).unwrap_or(0);
                }
                hp -= gear.attack_damage_from(monster);
                if hp <= 0 && !ignore_death {
                    break;
                }
            }
            if turns >= 100 {
                break;
            }
            turns += 1;
        }
        Fight {
            turns,
            hp,
            monster_hp,
            hp_lost: starting_hp - hp,
            result: if hp <= 0 || turns > MAX_TURN {
                FightResult::Loss
            } else {
                FightResult::Win
            },
            cd: Self::compute_cd(gear.haste(), turns),
        }
    }

    pub fn time_to_rest(health: i32) -> i32 {
        health / 5 + if health % 5 > 0 { 1 } else { 0 }
    }

    fn compute_cd(haste: i32, turns: i32) -> i32 {
        max(
            5,
            ((turns * 2) as f32 - (haste as f32 * 0.01) * (turns * 2) as f32).round() as i32,
        )
    }

    pub fn gather(skill_level: i32, resource_level: i32, cooldown_reduction: i32) -> i32 {
        ((25.0 - ((skill_level - resource_level) as f32 / 10.0))
            * (1.0 + cooldown_reduction as f32 / 100.0))
            .round() as i32
    }
}

#[derive(Debug)]
pub struct Fight {
    pub turns: i32,
    pub hp: i32,
    pub monster_hp: i32,
    pub hp_lost: i32,
    pub result: FightResult,
    pub cd: i32,
}

#[cfg(test)]
mod tests {
    use crate::{ITEMS, MONSTERS};

    use super::*;

    #[test]
    fn gather() {
        assert_eq!(Simulator::gather(17, 1, -10,), 21);
    }

    #[test]
    fn kill_deathnight() {
        let gear = Gear {
            weapon: ITEMS.get("skull_staff"),
            shield: ITEMS.get("steel_shield"),
            helmet: ITEMS.get("piggy_helmet"),
            body_armor: ITEMS.get("bandit_armor"),
            leg_armor: ITEMS.get("piggy_pants"),
            boots: ITEMS.get("adventurer_boots"),
            ring1: ITEMS.get("skull_ring"),
            ring2: ITEMS.get("skull_ring"),
            amulet: ITEMS.get("ruby_amulet"),
            artifact1: None,
            artifact2: None,
            artifact3: None,
            utility1: None,
            utility2: None,
        };
        let fight = Simulator::fight(30, 0, &gear, &MONSTERS.get("death_knight").unwrap(), false);
        println!("{:?}", fight);
        assert_eq!(fight.result, FightResult::Win);
    }

    #[test]
    fn kill_cultist_emperor() {
        let gear = Gear {
            weapon: ITEMS.get("magic_bow"),
            shield: ITEMS.get("gold_shield"),
            helmet: ITEMS.get("strangold_helmet"),
            body_armor: ITEMS.get("serpent_skin_armor"),
            leg_armor: ITEMS.get("strangold_legs_armor"),
            boots: ITEMS.get("gold_boots"),
            ring1: ITEMS.get("emerald_ring"),
            ring2: ITEMS.get("emerald_ring"),
            amulet: ITEMS.get("ancestral_talisman"),
            artifact1: ITEMS.get("christmas_star"),
            artifact2: None,
            artifact3: None,
            utility1: None,
            utility2: None,
        };
        let fight = Simulator::fight(
            40,
            0,
            &gear,
            &MONSTERS.get("cultist_emperor").unwrap(),
            false,
        );
        println!("{:?}", fight);
        assert_eq!(fight.result, FightResult::Win);
    }
}
