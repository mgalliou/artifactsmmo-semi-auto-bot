use crate::simulator::{DamageType, average_multiplier, crit_multiplier, critless_multiplier};

pub struct Hit {
    pub dmg: i32,
    pub r#type: DamageType,
    pub is_crit: bool,
}

impl Hit {
    pub fn new(
        attack_dmg: i32,
        dmg_increase: i32,
        target_res: i32,
        r#type: DamageType,
        is_crit: bool,
    ) -> Hit {
        let mut dmg = attack_dmg as f32;

        dmg *= if is_crit {
            crit_multiplier(dmg_increase, target_res)
        } else {
            critless_multiplier(dmg_increase, target_res)
        };
        Hit {
            dmg: dmg.round() as i32,
            r#type,
            is_crit,
        }
    }

    pub fn averaged(
        attack_dmg: i32,
        dmg_increase: i32,
        critical_strike: i32,
        target_res: i32,
        r#type: DamageType,
    ) -> Hit {
        let mut dmg = attack_dmg as f32;

        dmg *= average_multiplier(dmg_increase, critical_strike, target_res);
        Hit {
            r#type,
            dmg: dmg.round() as i32,
            is_crit: true,
        }
    }
}
