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
    ) -> Fight {
        let mut hp = 115 + 5 * level + gear.health_increase() - missing_hp;
        let mut monster_hp = monster.hp;
        let mut turns = 1;

        while turns <= 100 {
            if turns % 2 == 1 {
                monster_hp -= gear.attack_damage_against(monster).round() as i32;
                if monster_hp <= 0 {
                    break;
                }
            } else {
                hp -= gear.attack_damage_from(monster).round() as i32;
                if hp <= 0 {
                    break;
                }
            }
            turns += 1;
        }
        Fight {
            turns,
            hp_left: hp,
            result: if hp > 0 {
                FightResult::Win
            } else {
                FightResult::Loss
            },
            cd: ((turns * 2) as f32 - (gear.haste() as f32 * 0.01) * (turns * 2) as f32).ceil()
                as i32,
        }
    }

    pub fn gather(&self, skill_level: i32, ressource_level: i32, cooldown_reduction: i32) -> i32 {
        ((25.0 - ((skill_level - ressource_level) as f32 / 10.0))
            * (1.0 + cooldown_reduction as f32 / 100.0))
            .round() as i32
    }
}

pub struct Fight {
    pub turns: i32,
    pub hp_left: i32,
    pub result: FightResult,
    pub cd: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather() {
        let simulator = FightSimulator::new();

        assert_eq!(simulator.gather(17, 1, -10,), 21);
    }
}
