use strum_macros::{AsRefStr, Display, EnumIs, EnumIter, EnumString};

#[derive(Debug, Copy, Clone, PartialEq, Display, AsRefStr, EnumIter, EnumString, EnumIs)]
#[strum(serialize_all = "snake_case")]
pub enum EffectCode {
    CriticalStrike,
    Burn,
    Poison,
    Haste,
    Prospecting,
    Wisdom,
    Restore,
    Hp,
    BoostHp,
    Heal,
    Healing,
    Lifesteal,
    InventorySpace,

    AttackFire,
    AttackEarth,
    AttackWater,
    AttackAir,

    Dmg,
    DmgFire,
    DmgEarth,
    DmgWater,
    DmgAir,

    BoostDmgFire,
    BoostDmgEarth,
    BoostDmgWater,
    BoostDmgAir,
    ResDmgFire,
    ResDmgEarth,
    ResDmgWater,
    ResDmgAir,

    Mining,
    Woodcutting,
    Fishing,
    Alchemy,

    //Monster specific
    Reconstitution,
    Corrupted,
}

impl PartialEq<EffectCode> for String {
    fn eq(&self, other: &EffectCode) -> bool {
        other.as_ref() == *self
    }
}
