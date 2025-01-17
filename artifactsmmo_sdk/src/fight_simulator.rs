use crate::{
    consts::{BASE_HP, HP_PER_LEVEL, MAX_TURN},
    gear::Gear,
    items::ItemSchemaExt,
};
use artifactsmmo_openapi::models::{FightResult, MonsterSchema};
use std::cmp::max;

pub struct FightSimulator {}

impl Default for FightSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl FightSimulator {
    pub fn new() -> Self {
        Self {}
    }

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

    pub fn simulate(
        &self,
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
                    hp += gear.utility1.map(|u| u.restore()).unwrap_or(0);
                    hp += gear.utility2.map(|u| u.restore()).unwrap_or(0);
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

    pub fn compute_cd(haste: i32, turns: i32) -> i32 {
        max(
            5,
            ((turns * 2) as f32 - (haste as f32 * 0.01) * (turns * 2) as f32).round() as i32,
        )
    }

    pub fn gather(&self, skill_level: i32, resource_level: i32, cooldown_reduction: i32) -> i32 {
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
    use super::*;
    use crate::game::Game;

    #[test]
    fn gather() {
        let simulator = FightSimulator::new();

        assert_eq!(simulator.gather(17, 1, -10,), 21);
    }

    #[test]
    fn kill_deathnight() {
        let simulator = FightSimulator::new();
        let game = Game::new();
        let gear = Gear {
            weapon: game.items.get("skull_staff"),
            shield: game.items.get("steel_shield"),
            helmet: game.items.get("piggy_helmet"),
            body_armor: game.items.get("bandit_armor"),
            leg_armor: game.items.get("piggy_pants"),
            boots: game.items.get("adventurer_boots"),
            ring1: game.items.get("skull_ring"),
            ring2: game.items.get("skull_ring"),
            amulet: game.items.get("ruby_amulet"),
            artifact1: None,
            artifact2: None,
            artifact3: None,
            utility1: None,
            utility2: None,
        };
        let fight = simulator.simulate(
            30,
            0,
            &gear,
            game.monsters.get("death_knight").unwrap(),
            false,
        );
        println!("{:?}", fight);
        assert_eq!(fight.result, FightResult::Win);
    }

    #[test]
    fn kill_cultist_emperor() {
        let simulator = FightSimulator::new();
        let game = Game::new();
        let gear = Gear {
            weapon: game.items.get("magic_bow"),
            shield: game.items.get("gold_shield"),
            helmet: game.items.get("strangold_helmet"),
            body_armor: game.items.get("serpent_skin_armor"),
            leg_armor: game.items.get("strangold_legs_armor"),
            boots: game.items.get("gold_boots"),
            ring1: game.items.get("emerald_ring"),
            ring2: game.items.get("emerald_ring"),
            amulet: game.items.get("ancestral_talisman"),
            artifact1: game.items.get("christmas_star"),
            artifact2: None,
            artifact3: None,
            utility1: None,
            utility2: None,
        };
        let fight = simulator.simulate(
            40,
            0,
            &gear,
            game.monsters.get("cultist_emperor").unwrap(),
            false,
        );
        println!("{:?}", fight);
        assert_eq!(fight.result, FightResult::Win);
    }
}
