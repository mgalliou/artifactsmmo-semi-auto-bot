use crate::{
    CharacterClient, Gear, Level, Slot,
    entities::{Character, CharacterName, Monster},
    simulator::entity::{SimulationCharacter, SimulationEntity, SimulationMonster},
};
use itertools::Itertools;
use openapi::models::FightResult;
use rand::seq::IndexedRandom;
use std::cmp::max;

pub use damage_type::DamageType;
pub use effect_code::EffectCode;
pub use has_effects::HasEffects;
pub use hit::Hit;

mod entity;

pub mod damage_type;
pub mod effect_code;
pub mod has_effects;
pub mod hit;

const BASE_HP: u32 = 115;
const HP_PER_LEVEL: u32 = 5;
const BASE_INITIATIVE: i32 = 100;

const MAX_TURN: u32 = 100;
const SECOND_PER_TURN: u32 = 2;
const MIN_FIGHT_CD: u32 = 5;

const REST_HP_PER_SEC: u32 = 5;

const CRIT_MULTIPLIER: f32 = 0.5;
const BURN_MULTIPLIER: f32 = 0.90;

const THREAT_TARGET_CHANCE: u32 = 90;
const HEAL_INTERVAL: u32 = 3;

#[derive(Clone)]
pub struct FightSimulation {
    participants: Vec<Participant>,
    monster: Monster,
    params: FightParams,
}

impl FightSimulation {
    #[must_use]
    pub fn new(initiator: Participant, monster: Monster) -> Self {
        Self {
            participants: vec![initiator],
            monster,
            params: FightParams::default(),
        }
    }

    #[must_use]
    pub fn with_participants(mut self, participants: Vec<Participant>) -> Self {
        self.participants.extend(participants);
        self
    }

    #[must_use]
    pub const fn with_params(mut self, params: FightParams) -> Self {
        self.params = params;
        self
    }

    #[must_use]
    pub fn run(&self) -> FightReport {
        let chars = self
            .participants
            .iter()
            .map(SimulationCharacter::from)
            .collect_vec();
        let initiator = chars[0].clone();
        let mut monster = SimulationMonster::from(&self.monster);
        let mut fighters: Vec<Box<dyn SimulationEntity>> = vec![Box::new(monster.clone())];
        for char in &chars {
            fighters.push(Box::new(char.clone()));
        }
        let mut remaining_fighters = Vec::with_capacity(fighters.len());
        remaining_fighters.clone_from(&fighters);
        let mut turn = 1;
        while turn <= MAX_TURN && monster.is_alive() && chars.iter().any(SimulationEntity::is_alive)
        {
            if remaining_fighters.is_empty() {
                remaining_fighters.clone_from(&fighters);
            }
            let Some(mut next_fighter) = get_next_fighter(&remaining_fighters) else {
                break;
            };
            remaining_fighters.retain(|f| f.name() != next_fighter.name());
            if next_fighter.is_monster() {
                let Some(mut target) = pick_monster_target(&chars) else {
                    break;
                };
                monster.turn_against(&mut target, turn, self.params.averaged);
            } else {
                next_fighter.turn_against(&mut monster, turn, self.params.averaged);
            }
            turn += 1;
        }
        FightReport {
            turns: turn,
            hp: initiator.current_health(),
            hp_percent: initiator.health_percent(),
            hp_lost: initiator.starting_hp() - initiator.current_health(),
            monster_hp: monster.current_health(),
            result: if monster.is_dead() {
                FightResult::Win
            } else {
                FightResult::Loss
            },
            cd: compute_fight_cd(initiator.haste(), turn),
        }
    }

    #[must_use]
    pub fn win_rate(&self, samples: u32) -> f64 {
        let wins = (0..samples).filter(|_| self.run().is_winning()).count();
        wins as f64 / f64::from(samples)
    }
}

fn get_next_fighter(fighters: &[Box<dyn SimulationEntity>]) -> Option<Box<dyn SimulationEntity>> {
    fighters
        .iter()
        .filter(|f| f.is_alive())
        .max_set_by_key(|f| f.initiative())
        .into_iter()
        .max_set_by_key(|f| f.current_health())
        .choose(&mut rand::rng())
        .map(|&c| c.clone())
}

fn pick_monster_target(chars: &[SimulationCharacter]) -> Option<SimulationCharacter> {
    let chars_alive = chars.iter().filter(|c| c.is_alive()).collect_vec();
    if chars_alive.is_empty() {
        return None;
    }
    let use_threat = rand::random_ratio(THREAT_TARGET_CHANCE, 100);
    let targets = if use_threat {
        chars_alive.into_iter().max_set_by_key(HasEffects::threat)
    } else {
        chars_alive
    };
    targets
        .iter()
        .min_set_by_key(|c| c.current_health())
        .choose(&mut rand::rng())
        .map(|&&c| c.clone())
}

#[derive(Clone)]
pub struct Participant {
    name: CharacterName,
    level: u32,
    gear: Gear,
    utility1_quantity: u32,
    utility2_quantity: u32,
    missing_hp: i32,
}

impl Participant {
    #[must_use]
    pub fn new(name: CharacterName) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    #[must_use]
    pub fn with_gear(mut self, gear: Gear) -> Self {
        self.gear = gear;
        self
    }

    #[must_use]
    pub const fn with_utility1_quantity(mut self, quantity: u32) -> Self {
        self.utility1_quantity = quantity;
        self
    }

    #[must_use]
    pub const fn with_utility2_quantity(mut self, quantity: u32) -> Self {
        self.utility2_quantity = quantity;
        self
    }

    #[must_use]
    pub const fn with_missing_hp(mut self, hp: i32) -> Self {
        self.missing_hp = hp;
        self
    }
}

impl Default for Participant {
    fn default() -> Self {
        Self {
            name: "Participant".into(),
            level: 1,
            gear: Gear::default(),
            utility1_quantity: 100,
            utility2_quantity: 100,
            missing_hp: 0,
        }
    }
}

impl From<&CharacterClient> for Participant {
    fn from(value: &CharacterClient) -> Self {
        Self {
            name: value.name(),
            level: value.level(),
            gear: value.gear(),
            utility1_quantity: value.quantity_in_slot(Slot::Utility1),
            utility2_quantity: value.quantity_in_slot(Slot::Utility2),
            missing_hp: value.missing_hp(),
        }
    }
}

#[derive(Clone, Default)]
pub struct FightParams {
    averaged: bool,
    ignore_death: bool,
}

impl FightParams {
    #[must_use]
    pub const fn averaged() -> Self {
        Self {
            averaged: true,
            ignore_death: false,
        }
    }

    #[must_use]
    pub const fn ignore_death(mut self) -> Self {
        self.ignore_death = true;
        self
    }
}

#[derive(Debug)]
pub struct FightReport {
    pub turns: u32,
    pub hp: i32,
    pub hp_lost: i32,
    pub monster_hp: i32,
    pub result: FightResult,
    pub cd: u32,
    pub hp_percent: i32,
}

impl FightReport {
    #[must_use]
    pub const fn is_winning(&self) -> bool {
        matches!(self.result, FightResult::Win)
    }

    #[must_use]
    pub const fn is_losing(&self) -> bool {
        matches!(self.result, FightResult::Loss)
    }
}

/// Compute the average damage an attack will do against the given `target_resistance`.
#[inline]
#[must_use]
pub const fn average_dmg(
    attack_dmg: i32,
    dmg_increase: i32,
    critical_strike: i32,
    target_res: i32,
) -> f32 {
    attack_dmg as f32 * average_multiplier(dmg_increase, critical_strike, target_res)
}

const fn average_multiplier(dmg_increase: i32, critical_strike: i32, target_res: i32) -> f32 {
    critless_multiplier(dmg_increase, target_res)
        * (critical_strike as f32 * 0.01).mul_add(CRIT_MULTIPLIER, 1.0)
}

const fn critless_multiplier(dmg_increase: i32, target_res: i32) -> f32 {
    dmg_multiplier(dmg_increase) * res_multiplier(target_res)
}

const fn crit_multiplier(dmg_increase: i32, target_res: i32) -> f32 {
    critless_multiplier(dmg_increase, target_res) * (1.0 + CRIT_MULTIPLIER)
}

const fn dmg_multiplier(dmg_increase: i32) -> f32 {
    (dmg_increase as f32).mul_add(0.01, 1.0)
}

const fn res_multiplier(target_res: i32) -> f32 {
    if target_res > 100 {
        100.0
    } else {
        target_res as f32
    }
    .mul_add(-0.01, 1.0)
}

#[must_use]
pub fn time_to_rest(health: u32) -> u32 {
    health / REST_HP_PER_SEC + u32::from(!health.is_multiple_of(REST_HP_PER_SEC))
}

#[must_use]
pub fn compute_fight_cd(haste: i32, turns: u32) -> u32 {
    max(
        MIN_FIGHT_CD,
        (haste as f32 * 0.01)
            .mul_add(
                -((turns * SECOND_PER_TURN) as f32),
                (turns * SECOND_PER_TURN) as f32,
            )
            .round() as u32,
    )
}

#[must_use]
pub fn compute_gathering_cd(resource_level: u32, cooldown_reduction: i32) -> u32 {
    let level = resource_level as f32;
    let reduction = cooldown_reduction as f32;

    ((30.0 + (level / 2.0)) * reduction.mul_add(0.01, 1.0)).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_gather_cd() {
        assert_eq!(compute_gathering_cd(1, -10), 27);
    }

    #[test]
    fn gather_cd_zero_reduction() {
        assert_eq!(compute_gathering_cd(1, 0), 31);
    }

    #[test]
    fn gather_cd_high_level() {
        assert_eq!(compute_gathering_cd(10, 0), 35);
    }

    #[test]
    fn gather_cd_positive_reduction() {
        assert_eq!(compute_gathering_cd(10, 50), 53);
    }

    #[test]
    fn fight_cd_min() {
        assert_eq!(compute_fight_cd(0, 1), MIN_FIGHT_CD);
    }

    #[test]
    fn fight_cd_no_haste() {
        assert_eq!(compute_fight_cd(0, 10), 20);
    }

    #[test]
    fn fight_cd_with_haste() {
        assert_eq!(compute_fight_cd(10, 10), 18);
    }

    #[test]
    fn fight_cd_negative_haste() {
        assert_eq!(compute_fight_cd(-10, 10), 22);
    }

    #[test]
    fn average_dmg_zero() {
        assert!((average_dmg(0, 0, 0, 0).abs() < 0.001));
    }

    #[test]
    fn average_dmg_with_increase() {
        let dmg = average_dmg(10, 50, 0, 0);
        assert!((dmg - 15.0).abs() < 0.001);
    }

    #[test]
    fn average_dmg_with_crit() {
        let dmg = average_dmg(10, 0, 100, 0);
        assert!((dmg - 15.0).abs() < 0.001);
    }

    #[test]
    fn average_dmg_with_resistance() {
        let dmg = average_dmg(10, 0, 0, 50);
        assert!((dmg - 5.0).abs() < 0.001);
    }

    #[test]
    fn average_dmg_full_calculation() {
        let dmg = average_dmg(10, 50, 100, 50);
        assert!((dmg - 11.25).abs() < 0.001);
    }
}
