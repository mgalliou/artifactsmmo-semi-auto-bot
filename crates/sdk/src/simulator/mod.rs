use crate::{
    CharacterClient, Gear, Slot,
    character::HasCharacterData,
    entities::Monster,
    simulator::entity::{SimulationCharacter, SimulationEntity, SimulationMonster},
};
use openapi::models::FightResult;
use itertools::Itertools;
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

pub struct Simulator {}

impl Simulator {
    pub fn fight(
        initiator: Participant,
        participants: Option<Vec<Participant>>,
        monster: Monster,
        params: FightParams,
    ) -> Fight {
        let char = SimulationCharacter::new(
            initiator.name.clone(),
            initiator.level,
            initiator.gear,
            initiator.utility1_quantity,
            initiator.utility2_quantity,
            initiator.missing_hp,
            params.averaged,
        );
        let mut chars = vec![char.clone()];
        if let Some(participants) = participants {
            let participants = participants
                .into_iter()
                .map(|participant| {
                    SimulationCharacter::new(
                        participant.name.clone(),
                        participant.level,
                        participant.gear,
                        participant.utility1_quantity,
                        participant.utility2_quantity,
                        participant.missing_hp,
                        params.averaged,
                    )
                })
                .collect_vec();
            participants.iter().for_each(|p| chars.push(p.clone()));
        }
        let mut monster = SimulationMonster::new(monster, params.averaged);
        let mut fighters: Vec<Box<dyn SimulationEntity>> = vec![Box::new(monster.clone())];
        chars
            .iter()
            .for_each(|c| fighters.push(Box::new(c.clone())));
        let mut remaining_fighters = fighters.clone();
        let mut turn = 1;
        while turn <= MAX_TURN
            && monster.current_health() > 0
            && chars.iter().all(|c| c.current_health() > 0)
        {
            if remaining_fighters.is_empty() {
                remaining_fighters = fighters.clone();
            }
            let Some(mut fighter) = get_next_fighter(&mut remaining_fighters) else {
                break;
            };
            remaining_fighters.retain(|f| f.name() != fighter.name());
            if fighter.is_monster() {
                let Some(mut target) = pick_monster_target(&chars) else {
                    break;
                };
                monster.turn_against(&mut target, turn);
            } else {
                fighter.turn_against(&mut monster, turn);
            }
            turn += 1;
        }
        Fight {
            turns: turn,
            hp: char.current_health(),
            monster_hp: monster.current_health(),
            hp_lost: char.starting_hp() - char.current_health(),
            result: if char.current_health() <= 0
                || (turn > MAX_TURN && monster.current_health() > 0)
            {
                FightResult::Loss
            } else {
                FightResult::Win
            },
            cd: fight_cd(char.haste(), turn),
        }
    }
}

fn get_next_fighter(
    fighters: &mut Vec<Box<dyn SimulationEntity>>,
) -> Option<Box<dyn SimulationEntity>> {
    fighters
        .iter()
        .filter(|f| f.current_health() > 0)
        .max_set_by_key(|f| f.initiative())
        .into_iter()
        .max_set_by_key(|f| f.current_health())
        .choose(&mut rand::rng())
        .map(|&c| c.clone())
}

fn pick_monster_target(chars: &[SimulationCharacter]) -> Option<SimulationCharacter> {
    let chars_alive = chars.iter().filter(|c| c.current_health() > 0);
    let targets = if rand::random_range(1..=100) <= 90 {
        chars_alive.max_set_by_key(|c| c.threat())
    } else {
        chars_alive.collect_vec()
    };
    targets
        .iter()
        .min_set_by_key(|c| c.current_health())
        .choose(&mut rand::rng())
        .map(|&&c| c.clone())
}

pub struct Participant {
    name: String,
    level: u32,
    gear: Gear,
    utility1_quantity: u32,
    utility2_quantity: u32,
    missing_hp: i32,
}

impl Participant {
    pub fn new(
        name: String,
        level: u32,
        gear: Gear,
        utility1_quantity: u32,
        utility2_quantity: u32,
        missing_hp: i32,
    ) -> Self {
        Self {
            name,
            level,
            gear,
            utility1_quantity,
            utility2_quantity,
            missing_hp,
        }
    }
}

impl From<&CharacterClient> for Participant {
    fn from(value: &CharacterClient) -> Self {
        Self {
            name: value.name(),
            level: value.level(),
            gear: value.gear().clone(),
            utility1_quantity: value.quantity_in_slot(Slot::Utility1),
            utility2_quantity: value.quantity_in_slot(Slot::Utility2),
            missing_hp: value.missing_hp(),
        }
    }
}

#[derive(Default)]
pub struct FightParams {
    averaged: bool,
    ignore_death: bool,
}

impl FightParams {
    pub fn averaged(mut self) -> Self {
        self.averaged = true;
        self
    }

    pub fn ignore_death(mut self) -> Self {
        self.ignore_death = true;
        self
    }
}

#[derive(Debug)]
pub struct Fight {
    pub turns: u32,
    pub hp: i32,
    pub monster_hp: i32,
    pub hp_lost: i32,
    pub result: FightResult,
    pub cd: u32,
}

impl Fight {
    pub fn is_winning(&self) -> bool {
        matches!(self.result, FightResult::Win)
    }

    pub fn is_losing(&self) -> bool {
        matches!(self.result, FightResult::Loss)
    }
}

/// Compute the average damage an attack will do against the given `target_resistance`.
pub fn average_dmg(
    attack_dmg: i32,
    dmg_increase: i32,
    critical_strike: i32,
    target_res: i32,
) -> f32 {
    let multiplier = average_multiplier(dmg_increase, critical_strike, target_res);

    attack_dmg as f32 * multiplier
}

fn average_multiplier(dmg_increase: i32, critical_strike: i32, target_res: i32) -> f32 {
    critless_multiplier(dmg_increase, target_res)
        * (1.0 + critical_strike as f32 * 0.01 * CRIT_MULTIPLIER)
}

fn critless_multiplier(dmg_increase: i32, target_res: i32) -> f32 {
    dmg_multiplier(dmg_increase) * res_multiplier(target_res)
}

fn crit_multiplier(dmg_increase: i32, target_res: i32) -> f32 {
    critless_multiplier(dmg_increase, target_res) * (1.0 + CRIT_MULTIPLIER)
}

fn dmg_multiplier(dmg_increase: i32) -> f32 {
    1.0 + dmg_increase as f32 * 0.01
}

fn res_multiplier(target_res: i32) -> f32 {
    let target_res = if target_res > 100 {
        100.0
    } else {
        target_res as f32
    };
    1.0 - target_res * 0.01
}

pub fn time_to_rest(health: u32) -> u32 {
    health / REST_HP_PER_SEC
        + if health.is_multiple_of(REST_HP_PER_SEC) {
            0
        } else {
            1
        }
}

fn fight_cd(haste: i32, turns: u32) -> u32 {
    max(
        MIN_FIGHT_CD,
        ((turns * SECOND_PER_TURN) as f32
            - (haste as f32 * 0.01) * (turns * SECOND_PER_TURN) as f32)
            .round() as u32,
    )
}

pub fn gather_cd(resource_level: u32, cooldown_reduction: i32) -> u32 {
    let level = resource_level as f32;
    let reduction = cooldown_reduction as f32;

    ((30.0 + (level / 2.0)) * (1.0 + reduction * 0.01)).round() as u32
}

#[cfg(test)]
mod tests {
    use crate::simulator::gather_cd;

    //TODO: rewrite tests
    // use crate::{ITEMS, MONSTERS};
    //
    // use super::*;
    //
    // #[test]
    // fn gather() {
    //     assert_eq!(Simulator::gather(17, 1, -10,), 21);
    // }
    //
    // #[test]
    // fn kill_deathnight() {
    //     let gear = Gear {
    //         weapon: ITEMS.get("skull_staff"),
    //         shield: ITEMS.get("steel_shield"),
    //         helmet: ITEMS.get("piggy_helmet"),
    //         body_armor: ITEMS.get("bandit_armor"),
    //         leg_armor: ITEMS.get("piggy_pants"),
    //         boots: ITEMS.get("adventurer_boots"),
    //         ring1: ITEMS.get("skull_ring"),
    //         ring2: ITEMS.get("skull_ring"),
    //         amulet: ITEMS.get("ruby_amulet"),
    //         artifact1: None,
    //         artifact2: None,
    //         artifact3: None,
    //         utility1: None,
    //         utility2: None,
    //     };
    //     let fight = Simulator::fight(30, 0, &gear, &MONSTERS.get("death_knight").unwrap(), false);
    //     println!("{:?}", fight);
    //     assert_eq!(fight.result, FightResult::Win);
    // }
    //
    // #[test]
    // fn kill_cultist_emperor() {
    //     let gear = Gear {
    //         weapon: ITEMS.get("magic_bow"),
    //         shield: ITEMS.get("gold_shield"),
    //         helmet: ITEMS.get("strangold_helmet"),
    //         body_armor: ITEMS.get("serpent_skin_armor"),
    //         leg_armor: ITEMS.get("strangold_legs_armor"),
    //         boots: ITEMS.get("gold_boots"),
    //         ring1: ITEMS.get("emerald_ring"),
    //         ring2: ITEMS.get("emerald_ring"),
    //         amulet: ITEMS.get("ancestral_talisman"),
    //         artifact1: ITEMS.get("christmas_star"),
    //         artifact2: None,
    //         artifact3: None,
    //         utility1: None,
    //         utility2: None,
    //     };
    //     let fight = Simulator::fight(
    //         40,
    //         0,
    //         &gear,
    //         &MONSTERS.get("cultist_emperor").unwrap(),
    //         false,
    //     );
    //     println!("{:?}", fight);
    //     assert_eq!(fight.result, FightResult::Win);
    // }
    #[test]
    fn check_gather_cd() {
        assert_eq!(gather_cd(1, -10), 27)
    }
}
