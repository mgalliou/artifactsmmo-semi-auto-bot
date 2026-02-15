use crate::{
    Gear,
    entities::{Item, Monster},
    simulator::{
        BASE_HP, BASE_INITIATIVE, BURN_MULTIPLIER, HP_PER_LEVEL, HasEffects,
        damage_type::DamageType,
    },
};
use dyn_clone::DynClone;
use openapi::models::SimpleEffectSchema;
use std::{cell::RefCell, rc::Rc};

pub(super) trait SimulationEntity: HasEffects + DynClone {
    fn turn_against(&mut self, target: &mut dyn SimulationEntity, turn: u32) {
        if turn == self.reconstitution() as u32 {
            self.set_health(self.max_hp());
        }
        if self.current_health() < self.max_hp() / 2 {
            self.consume_restore_utilities();
        }
        if self.current_turn().is_multiple_of(3) {
            self.receive_healing();
        }
        if self.burning() > 0 {
            self.suffer_burning();
            if self.current_health() < 1 {
                return;
            }
        }
        if self.poisoned() > 0 {
            self.suffer_poisoning();
            if self.current_health() < 1 {
                return;
            }
        }
        if self.current_turn() == 1 {
            self.apply_burn(target);
            self.apply_poison(target);
        }
        for hit in self.hits_against(target, self.averaged()).iter() {
            target.dec_health(hit.dmg);
            if hit.is_crit {
                self.inc_health(hit.dmg * self.lifesteal() / 100);
            }
            if target.current_health() < 1 {
                return;
            }
            if target.corrupted() > 0 {
                target.suffer_corruption(hit.r#type);
            }
        }
        self.inc_turn();
    }

    fn apply_burn(&self, target: &mut dyn SimulationEntity) {
        target.set_burning(self.critless_dmg_against(target) * self.burn() / 100);
    }

    fn apply_poison(&self, target: &mut dyn SimulationEntity) {
        target.set_poisoned(self.poison());
    }

    fn receive_healing(&mut self) {
        self.inc_health((self.max_hp() as f32 * self.healing() as f32 * 0.01).round() as i32)
    }

    fn consume_restore_utilities(&mut self) {
        if let Some(utility1) = self.utility1()
            && self.utility1_quantity() > 0
        {
            let restore = utility1.restore();
            if restore > 0 {
                self.inc_health(restore);
                self.dec_utility1();
            }
        }
        if let Some(utility2) = self.utility2()
            && self.utility2_quantity() > 0
        {
            let restore = utility2.restore();
            if restore > 0 {
                self.inc_health(restore);
                self.dec_utility2();
            }
        }
    }

    fn suffer_burning(&mut self) {
        self.set_burning((self.burning() as f32 * BURN_MULTIPLIER).round() as i32);
        self.dec_health(self.burning());
    }

    fn suffer_poisoning(&mut self) {
        self.dec_health(self.poisoned());
    }

    fn name(&self) -> String;
    fn averaged(&self) -> bool;
    fn current_turn(&self) -> u32;
    fn inc_turn(&mut self);
    fn starting_hp(&self) -> i32;
    fn max_hp(&self) -> i32;
    fn current_health(&self) -> i32;
    fn set_health(&mut self, value: i32);
    fn burning(&self) -> i32;
    fn set_burning(&mut self, value: i32);
    fn poisoned(&self) -> i32;
    fn set_poisoned(&mut self, value: i32);
    fn suffer_corruption(&mut self, r#type: DamageType);

    fn inc_health(&mut self, value: i32) {
        let missing = self.max_hp() - self.current_health();
        let value = if value > missing { missing } else { value };
        self.set_health(self.current_health() + value)
    }

    fn dec_health(&mut self, value: i32) {
        self.set_health(self.current_health() - value)
    }

    fn utility1(&self) -> Option<Item> {
        None
    }

    fn utility2(&self) -> Option<Item> {
        None
    }

    fn utility1_quantity(&self) -> u32 {
        0
    }

    fn utility2_quantity(&self) -> u32 {
        0
    }

    fn dec_utility1(&mut self) {}
    fn dec_utility2(&mut self) {}
    fn is_monster(&self) -> bool;
}

dyn_clone::clone_trait_object!(SimulationEntity);

#[derive(Clone)]
pub(super) struct SimulationCharacter(Rc<RefCell<BaseSimulationCharacter>>);

impl SimulationCharacter {
    pub(super) fn new(
        name: String,
        level: u32,
        gear: Gear,
        utility1_quantity: u32,
        utility2_quantity: u32,
        missing_hp: i32,
        average: bool,
    ) -> Self {
        Self(Rc::new(RefCell::new(BaseSimulationCharacter::new(
            name,
            level,
            gear,
            utility1_quantity,
            utility2_quantity,
            missing_hp,
            average,
        ))))
    }
}

pub struct BaseSimulationCharacter {
    name: String,
    averaged: bool,
    gear: Gear,
    pub(super) starting_hp: i32,
    max_hp: i32,
    inititive: i32,

    current_turn: u32,
    pub(super) current_health: i32,

    fire_res: i32,
    earth_res: i32,
    water_res: i32,
    air_res: i32,

    burning: i32,
    poisoned: i32,

    utility1_quantity: u32,
    utility2_quantity: u32,
}

impl BaseSimulationCharacter {
    pub(super) fn new(
        name: String,
        level: u32,
        gear: Gear,
        utility1_quantity: u32,
        utility2_quantity: u32,
        missing_hp: i32,
        average: bool,
    ) -> Self {
        let base_hp = (BASE_HP + HP_PER_LEVEL * level) as i32;
        let max_hp = base_hp + gear.health();
        let starting_hp = max_hp - missing_hp;
        Self {
            name,
            max_hp,
            starting_hp,
            inititive: BASE_INITIATIVE + gear.initiative(),
            current_health: starting_hp,
            current_turn: 1,
            fire_res: gear.res(DamageType::Fire),
            earth_res: gear.res(DamageType::Earth),
            water_res: gear.res(DamageType::Water),
            air_res: gear.res(DamageType::Air),
            gear,
            utility1_quantity,
            utility2_quantity,
            burning: 0,
            poisoned: 0,
            averaged: average,
        }
    }
}

impl SimulationEntity for SimulationCharacter {
    fn name(&self) -> String {
        self.0.borrow().name.clone()
    }

    fn averaged(&self) -> bool {
        self.0.borrow().averaged
    }

    fn current_turn(&self) -> u32 {
        self.0.borrow().current_turn
    }

    fn inc_turn(&mut self) {
        self.0.borrow_mut().current_turn += 1
    }

    fn max_hp(&self) -> i32 {
        self.0.borrow().max_hp
    }

    fn current_health(&self) -> i32 {
        self.0.borrow().current_health
    }

    fn set_health(&mut self, value: i32) {
        self.0.borrow_mut().current_health = value;
    }

    fn burning(&self) -> i32 {
        self.0.borrow().burning
    }

    fn set_burning(&mut self, value: i32) {
        self.0.borrow_mut().burning = value;
    }

    fn poisoned(&self) -> i32 {
        self.0.borrow().poisoned
    }

    fn set_poisoned(&mut self, value: i32) {
        self.0.borrow_mut().poisoned = value;
    }

    fn suffer_corruption(&mut self, r#type: DamageType) {
        let corrupted = self.corrupted();
        match r#type {
            DamageType::Fire => self.0.borrow_mut().fire_res -= corrupted,
            DamageType::Earth => self.0.borrow_mut().earth_res -= corrupted,
            DamageType::Water => self.0.borrow_mut().water_res -= corrupted,
            DamageType::Air => self.0.borrow_mut().air_res -= corrupted,
        }
    }

    fn utility1(&self) -> Option<Item> {
        self.0.borrow().gear.utility1.clone()
    }

    fn utility2(&self) -> Option<Item> {
        self.0.borrow().gear.utility2.clone()
    }

    fn utility1_quantity(&self) -> u32 {
        self.0.borrow().utility1_quantity
    }

    fn utility2_quantity(&self) -> u32 {
        self.0.borrow().utility2_quantity
    }

    fn dec_utility1(&mut self) {
        self.0.borrow_mut().utility1_quantity = self.utility1_quantity().saturating_sub(1);
    }

    fn dec_utility2(&mut self) {
        self.0.borrow_mut().utility2_quantity = self.utility2_quantity().saturating_sub(1);
    }

    fn is_monster(&self) -> bool {
        false
    }

    fn starting_hp(&self) -> i32 {
        self.0.borrow().starting_hp
    }
}

impl HasEffects for SimulationCharacter {
    fn res(&self, r#type: DamageType) -> i32 {
        let inner = self.0.borrow();
        match r#type {
            DamageType::Fire => inner.fire_res,
            DamageType::Earth => inner.earth_res,
            DamageType::Water => inner.water_res,
            DamageType::Air => inner.air_res,
        }
    }

    fn initiative(&self) -> i32 {
        self.0.borrow().inititive
    }

    fn effect_value(&self, effect: &str) -> i32 {
        self.0.borrow().gear.effect_value(effect)
    }

    fn effects(&self) -> Vec<SimpleEffectSchema> {
        self.0.borrow().gear.effects().clone()
    }
}

#[derive(Clone)]
pub(super) struct SimulationMonster(Rc<RefCell<BaseSimulationMonster>>);

impl SimulationMonster {
    pub(super) fn new(monster: Monster, average: bool) -> Self {
        Self(Rc::new(RefCell::new(BaseSimulationMonster::new(
            monster, average,
        ))))
    }
}

pub(super) struct BaseSimulationMonster {
    averaged: bool,
    monster: Monster,

    current_turn: u32,
    pub(super) current_health: i32,

    fire_res: i32,
    earth_res: i32,
    water_res: i32,
    air_res: i32,

    burning: i32,
    poisoned: i32,
}

impl BaseSimulationMonster {
    pub(super) fn new(monster: Monster, average: bool) -> Self {
        Self {
            current_health: monster.health(),
            current_turn: 1,
            burning: 0,
            poisoned: 0,
            averaged: average,
            fire_res: monster.res(DamageType::Fire),
            earth_res: monster.res(DamageType::Earth),
            water_res: monster.res(DamageType::Water),
            air_res: monster.res(DamageType::Air),
            monster,
        }
    }
}

impl SimulationEntity for SimulationMonster {
    fn name(&self) -> String {
        self.0.borrow().monster.name().to_string()
    }

    fn averaged(&self) -> bool {
        self.0.borrow().averaged
    }

    fn current_turn(&self) -> u32 {
        self.0.borrow().current_turn
    }

    fn inc_turn(&mut self) {
        self.0.borrow_mut().current_turn += 1
    }

    fn max_hp(&self) -> i32 {
        self.0.borrow().monster.health()
    }

    fn current_health(&self) -> i32 {
        self.0.borrow().current_health
    }

    fn set_health(&mut self, value: i32) {
        self.0.borrow_mut().current_health = value;
    }

    fn burning(&self) -> i32 {
        self.0.borrow().burning
    }

    fn set_burning(&mut self, value: i32) {
        self.0.borrow_mut().burning = value
    }

    fn poisoned(&self) -> i32 {
        self.0.borrow().poisoned
    }

    fn set_poisoned(&mut self, value: i32) {
        self.0.borrow_mut().poisoned = value
    }

    fn suffer_corruption(&mut self, r#type: DamageType) {
        let corrupted = self.corrupted();
        let mut inner = self.0.borrow_mut();
        match r#type {
            DamageType::Fire => inner.fire_res -= corrupted,
            DamageType::Earth => inner.earth_res -= corrupted,
            DamageType::Water => inner.water_res -= corrupted,
            DamageType::Air => inner.air_res -= corrupted,
        }
    }

    fn is_monster(&self) -> bool {
        true
    }

    fn starting_hp(&self) -> i32 {
        self.max_hp()
    }
}

impl HasEffects for SimulationMonster {
    fn health(&self) -> i32 {
        self.0.borrow().monster.health()
    }

    fn attack_dmg(&self, r#type: DamageType) -> i32 {
        self.0.borrow().monster.attack_dmg(r#type)
    }

    fn critical_strike(&self) -> i32 {
        self.0.borrow().monster.critical_strike()
    }

    fn res(&self, r#type: DamageType) -> i32 {
        let inner = self.0.borrow();
        match r#type {
            DamageType::Fire => inner.fire_res,
            DamageType::Earth => inner.earth_res,
            DamageType::Water => inner.water_res,
            DamageType::Air => inner.air_res,
        }
    }

    fn effects(&self) -> Vec<SimpleEffectSchema> {
        self.0.borrow().monster.effects().clone()
    }
}
