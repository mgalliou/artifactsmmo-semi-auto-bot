use super::gear::Gear;
use artifactsmmo_openapi::models::{FightResult, MonsterSchema};

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

    pub fn simulate(&self, level: i32, gear: &Gear, monster: &MonsterSchema) -> Fight {
        let mut hp = 115 + 5 * level + gear.health_increase();
        let mut monster_hp = monster.hp;
        let mut turns = 1;

        while turns <= 100 {
            monster_hp -= gear.attack_damage_against(monster).floor() as i32;
            if monster_hp <= 0 {
                break;
            }
            hp -= gear.attack_damage_from(monster).floor() as i32;
            if hp <= 0 {
                break;
            }
            turns += 1;
        }
        Fight {
            turns,
            result: if monster_hp <= 0 {
                FightResult::Win
            } else {
                FightResult::Loss
            },
        }
    }
}

pub struct Fight {
    pub turns: i32,
    pub result: FightResult,
}
