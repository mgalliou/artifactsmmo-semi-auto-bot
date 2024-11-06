use super::{gear::Gear, items::Items, monsters::Monsters};
use artifactsmmo_openapi::models::{fight_schema::Result, MonsterSchema};
use std::sync::Arc;

pub struct FightSimulator {
    items: Arc<Items>,
    monsters: Arc<Monsters>,
}

impl FightSimulator {
    pub fn new(items: &Arc<Items>, monsters: &Arc<Monsters>) -> Self {
        Self {
            items: items.clone(),
            monsters: monsters.clone(),
        }
    }

    pub fn simulate(&self, level: i32, equipment: &Gear, monster: &MonsterSchema) -> Fight {
        let mut hp = 115 + 5 * level + equipment.health_increase();
        let mut monster_hp = monster.hp;
        let mut turns = 1;

        while turns <= 100 {
            monster_hp -= equipment.attack_damage_against(monster).floor() as i32;
            if monster_hp <= 0 {
                break;
            }
            hp -= equipment.attack_damage_from(monster).floor() as i32;
            if hp <= 0 {
                break;
            }
            turns += 1;
        }
        Fight {
            turns,
            result: if monster_hp <= 0 {
                Result::Win
            } else {
                Result::Lose
            },
        }
    }
}

pub struct Fight {
    pub turns: i32,
    pub result: Result,
}
