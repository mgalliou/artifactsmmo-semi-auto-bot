use super::{
    account::Account,
    api::my_character::MyCharacterApi,
    bank::Bank,
    char_config::CharConfig,
    items::{DamageType, Items, Slot, Type},
    maps::Maps,
    monsters::Monsters,
    resources::Resources,
    skill::Skill,
    ItemSchemaExt, MapSchemaExt, MonsterSchemaExt,
};
use artifactsmmo_openapi::models::{
    CharacterSchema, InventorySlot, ItemSchema, MapSchema, MonsterSchema, ResourceSchema,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, info, warn};
use serde::Deserialize;
use std::{
    cmp::Ordering,
    io,
    option::Option,
    sync::{Arc, RwLock},
    thread::{self, sleep, JoinHandle},
    time::Duration,
    vec::Vec,
};
use strum::IntoEnumIterator;
mod actions;
use ordered_float::OrderedFloat;

pub struct Character {
    name: String,
    my_api: MyCharacterApi,
    account: Account,
    maps: Arc<Maps>,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    items: Arc<Items>,
    bank: Arc<RwLock<Bank>>,
    pub conf: Arc<RwLock<CharConfig>>,
    pub data: Arc<RwLock<CharacterSchema>>,
}

impl Character {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account: &Account,
        maps: Arc<Maps>,
        resources: Arc<Resources>,
        monsters: Arc<Monsters>,
        items: Arc<Items>,
        bank: Arc<RwLock<Bank>>,
        conf: Arc<RwLock<CharConfig>>,
        data: Arc<RwLock<CharacterSchema>>,
    ) -> Character {
        Character {
            name: data.read().map(|d| d.name.to_owned()).unwrap(),
            conf,
            my_api: MyCharacterApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            account: account.clone(),
            maps,
            resources,
            monsters,
            items,
            bank,
            data,
        }
    }

    pub fn run(char: Character) -> Result<JoinHandle<()>, io::Error> {
        thread::Builder::new()
            .name(char.data().name.to_owned())
            .spawn(move || {
                char.run2();
            })
    }

    fn run2(&self) {
        info!("{}: started !", self.name);
        if Role::Fighter != self.conf().role
            && self
                .equipment_in(Slot::Weapon)
                .is_some_and(|w| w.code == "wooden_stick")
        {
            let _ = self.action_unequip(Slot::Weapon);
            self.deposit_all();
        };
        while self.conf.read().is_ok_and(|c| c.role != Role::Idle) {
            self.process_inventory();
            self.process_task();
            if let Some(skill) = self.target_skill_to_level() {
                self.levelup_by_crafting(skill);
            } else if let Some(craft) = self.conf().target_craft {
                self.craft_all_from_bank(&craft);
            } else if let Some(monster) = self.target_monster() {
                self.improve_equipment(monster);
                self.kill_monster(&monster.code);
            } else if let Some(resource) = self.target_resource() {
                self.gather_resource(&resource.code);
            }
        }
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }

    fn data(&self) -> CharacterSchema {
        self.data.read().unwrap().clone()
    }

    fn process_inventory(&self) {
        if self.inventory_is_full() {
            if self.conf().process_gathered {
                self.process_raw_mats();
            }
            self.deposit_all_mats();
            self.deposit_all_consumables();
        }
    }

    fn process_task(&self) {
        if self.data().task.is_empty() || self.task_finished() {
            if self.task_finished() {
                let _ = self.action_complete_task();
            }
            let _ = self.action_accept_task();
        }
    }

    /// Process the raw materials in the Character inventory by converting the
    /// materials having only one possible receipe, and depositing the crafted
    /// items.
    fn process_raw_mats(&self) {
        let unique_crafts = self
            .inventory_raw_mats()
            .into_iter()
            .filter_map(|rm| self.items.unique_craft(&rm.code))
            .filter(|cw| self.has_mats_for(&cw.code) > 0)
            .collect_vec();
        unique_crafts.iter().for_each(|p| {
            self.craft_all(&p.code);
        });
        unique_crafts
            .iter()
            .for_each(|p| self.deposit_all_of(&p.code));
    }

    fn inventory_raw_mats(&self) -> Vec<&ItemSchema> {
        self.data
            .read()
            .map(|d| {
                d.inventory
                    .iter()
                    .flatten()
                    .filter_map(|slot| self.items.get(&slot.code))
                    .filter(|i| i.is_raw_mat())
                    .collect_vec()
            })
            .into_iter()
            .flatten()
            .collect_vec()
    }

    fn kill_monster(&self, code: &str) -> bool {
        if let Some(map) = self.closest_map_with_resource(code) {
            return self.action_move(map.x, map.y) && self.action_fight().is_ok();
        }
        false
    }

    fn can_kill(&self, monster: &MonsterSchema) -> bool {
        let turns_to_kill = monster.hp as f32 / self.attack_damage_against(monster);
        let turns_to_be_killed = self
            .data
            .read()
            .map_or(0.0, |d| d.hp as f32 / self.attack_damage_from(monster));
        turns_to_be_killed < turns_to_kill
    }

    fn gather_resource(&self, code: &str) -> bool {
        if let Some(map) = self.closest_map_with_resource(code) {
            return self.action_move(map.x, map.y) && self.action_gather().is_ok();
        }
        false
    }

    fn target_skill_to_level(&self) -> Option<Skill> {
        let mut skills = vec![];
        if self.conf().weaponcraft {
            skills.push(Skill::Weaponcrafting);
        }
        if self.conf().gearcraft {
            skills.push(Skill::Gearcrafting);
        }
        if self.conf().jewelcraft {
            skills.push(Skill::Jewelrycrafting);
        }
        if self.conf().cook {
            skills.push(Skill::Cooking);
        }
        skills.sort_by_key(|s| self.skill_level(*s));
        skills.into_iter().find(|&skill| {
            self.items
                .best_for_leveling(self.skill_level(skill), skill)
                .is_some_and(|i| self.bank.read().is_ok_and(|b| b.has_mats_for(&i.code) > 0))
        })
    }

    fn target_monster(&self) -> Option<&MonsterSchema> {
        if self.conf().role == Role::Fighter {
            if self.conf().do_tasks && self.data().task_type == "monsters" && !self.task_finished()
            {
                return self.monsters.get(&self.data().task);
            } else if let Some(monster) = &self.conf().fight_target {
                return self.monsters.get(monster);
            } else {
                return self.monsters.lowest_providing_exp(self.data().level);
            }
        }
        None
    }

    fn target_resource(&self) -> Option<&ResourceSchema> {
        match self.conf().role {
            Role::Miner | Role::Woodcutter | Role::Fisher => {
                if let Some(item) = &self.conf().target_item {
                    return self
                        .resources
                        .dropping(item)
                        .iter()
                        .min_by_key(|r| r.level)
                        .copied();
                } else if let Some(skill) = self.conf().role.to_skill() {
                    return self
                        .resources
                        .lowest_providing_exp(self.skill_level(skill), skill);
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn task_finished(&self) -> bool {
        self.data().task_progress >= self.data().task_total
    }

    fn equipment_in(&self, slot: Slot) -> Option<&ItemSchema> {
        let binding = self.data();
        let code = match slot {
            Slot::Weapon => &binding.weapon_slot,
            Slot::Shield => &binding.shield_slot,
            Slot::Helmet => &binding.helmet_slot,
            Slot::BodyArmor => &binding.body_armor_slot,
            Slot::LegArmor => &binding.leg_armor_slot,
            Slot::Boots => &binding.boots_slot,
            Slot::Ring1 => &binding.ring1_slot,
            Slot::Ring2 => &binding.ring2_slot,
            Slot::Amulet => &binding.amulet_slot,
            Slot::Artifact1 => &binding.artifact1_slot,
            Slot::Artifact2 => &binding.artifact2_slot,
            Slot::Artifact3 => &binding.artifact3_slot,
            Slot::Consumable1 => &binding.consumable1_slot,
            Slot::Consumable2 => &binding.consumable2_slot,
        };
        self.items.get(code)
    }

    //TODO: handle item already in inventory
    fn levelup_by_crafting(&self, skill: Skill) -> bool {
        info!("{} leveling {:#?} by crafting.", self.name, skill);
        let mut crafted_once = false;
        if let Some(best) = self.items.best_for_leveling(self.skill_level(skill), skill) {
            self.withdraw_max_mats_for(&best.code);
            let mut crafted = -1;
            while self.skill_level(skill) - best.level <= 10 && crafted != 0 {
                crafted_once = true;
                // TODO ge prices handling
                crafted = self.craft_all(&best.code);
                let _ = self.action_recycle(&best.code, crafted);
            }
            self.deposit_all_mats();
        }
        crafted_once
    }

    fn craft_all_from_bank(&self, code: &str) -> i32 {
        debug!("{}: crafting all '{}' from bank.", self.name, code);
        if self.bank.read().is_ok_and(|b| b.has_mats_for(code) > 0) {
            self.deposit_all();
            self.withdraw_max_mats_for(code);
            return self.craft_all(code);
        }
        0
    }

    fn skill_level(&self, skill: Skill) -> i32 {
        match skill {
            Skill::Cooking => self.data().cooking_level,
            Skill::Fishing => self.data().fishing_level,
            Skill::Gearcrafting => self.data().gearcrafting_level,
            Skill::Jewelrycrafting => self.data().jewelrycrafting_level,
            Skill::Mining => self.data().mining_level,
            Skill::Weaponcrafting => self.data().weaponcrafting_level,
            Skill::Woodcutting => self.data().woodcutting_level,
        }
    }

    /// Returns a copy of the inventory to be used while depositing or
    /// withdrawing items.
    fn inventory_copy(&self) -> Vec<InventorySlot> {
        self.data()
            .inventory
            .iter()
            .flatten()
            .cloned()
            .collect_vec()
    }

    fn deposit_all_mats(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!("{}: depositing all materials to the bank.", self.name);
        for slot in self.inventory_copy() {
            if slot.quantity > 0 && self.items.is_of_type(&slot.code, Type::Resource) {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    fn deposit_all_consumables(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!("{}: depositing all consumables to the bank.", self.name);
        for slot in self.inventory_copy() {
            if slot.quantity > 0 && self.items.is_of_type(&slot.code, Type::Consumable) {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    fn deposit_all(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!("{} depositing all items to the bank.", self.name);
        for slot in self.inventory_copy() {
            if slot.quantity > 0 {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    fn deposit_all_of(&self, code: &str) {
        let amount = self.amount_in_inventory(code);
        if amount > 0 {
            let _ = self.action_deposit(code, amount);
        }
    }

    /// Withdraw the materials required to craft the `quantity` of the
    /// item `code` and returns the maximum amount that can be crafted.
    fn withdraw_mats_for(&self, code: &str, quantity: i32) -> bool {
        info!(
            "{}: withdrawing materials for '{} x{}'.",
            self.name, code, quantity
        );
        let mats = self.items.mats(code);
        for mat in &mats {
            if !self
                .bank
                .read()
                .is_ok_and(|b| b.has_item(&mat.code) >= mat.quantity * quantity)
            {
                warn!("{}: not enough resources in bank to withdraw the materials required to craft [{code}] * {quantity}", self.name);
                return false;
            }
        }
        for mat in &mats {
            let _ = self.action_withdraw(&mat.code, mat.quantity * quantity);
        }
        true
    }

    /// Withdraw the maximum amount of materials to craft the maximum amount of
    /// the item `code` and returns the maximum amount that can be crafted.
    fn withdraw_max_mats_for(&self, code: &str) -> i32 {
        info!(
            "{}: getting maximum amount of materials in bank to craft '{}'.",
            self.name, code
        );
        let can_carry = self.inventory_free_space() / self.items.mats_quantity_for(code);
        let can_craft_from_bank = self.bank.read().map_or(0, |b| b.has_mats_for(code));
        let max = if can_craft_from_bank < can_carry {
            can_craft_from_bank
        } else {
            can_carry
        };
        self.withdraw_mats_for(code, max);
        max
    }

    /// Craft the maximum amount of the item `code` with the materials currently available
    /// in the character inventory and returns the amount crafted.
    fn craft_all(&self, code: &str) -> i32 {
        info!("{}: going to crafting all '{}'.", self.name, code);
        let n = self.has_mats_for(code);
        if n > 0 && self.action_craft(code, n).is_ok() {
            info!("{} crafted all {} ({})", self.name, code, n);
        }
        n
    }

    /// Craft the maximum amount of the item `code` with the items  currently
    /// available in the character inventory and returns the amount recycled.
    fn recycle_all(&self, code: &str) -> i32 {
        info!("{}: recycling all '{}'.", self.name, code);
        let item = self.inventory_copy().into_iter().find(|i| i.code == code);
        item.map_or(0, |i| {
            if self.action_recycle(&i.code, i.quantity).is_ok() {
                i.quantity
            } else {
                0
            }
        })
    }

    fn move_to_bank(&self) {
        let _ = self.action_move(4, 1);
    }

    fn wait_for_cooldown(&self) {
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        info!(
            "{}: cooling down for {}.{} secondes",
            self.name,
            s.as_secs(),
            s.subsec_millis()
        );
        sleep(s);
    }

    fn remaining_cooldown(&self) -> Duration {
        if let Some(exp) = self.cooldown_expiration() {
            let synced = Utc::now() - self.account.server_offset;
            if synced.cmp(&exp.to_utc()) == Ordering::Less {
                return (exp.to_utc() - synced).to_std().unwrap();
            }
        }
        Duration::from_secs(0)
    }

    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.data()
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }

    fn inventory_is_full(&self) -> bool {
        self.inventory_total() == self.data().inventory_max_items
            || self
                .data
                .read()
                .is_ok_and(|d| d.inventory.iter().flatten().all(|s| s.quantity > 0))
    }

    fn amount_in_inventory(&self, code: &str) -> i32 {
        self.data()
            .inventory
            .iter()
            .flatten()
            .find(|i| i.code == code)
            .map_or(0, |i| i.quantity)
    }

    fn inventory_free_space(&self) -> i32 {
        self.data().inventory_max_items - self.inventory_total()
    }

    fn inventory_total(&self) -> i32 {
        self.data()
            .inventory
            .map_or(0, |inv| inv.iter().map(|i| i.quantity).sum())
    }

    fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats(code)
            .iter()
            .filter(|mat| mat.quantity > 0)
            .map(|mat| self.amount_in_inventory(&mat.code) / mat.quantity)
            .min()
            .unwrap_or(0)
    }

    fn closest_map_among<'a>(&'a self, maps: Vec<&'a MapSchema>) -> Option<&MapSchema> {
        Maps::closest_from_amoung(self.data().x, self.data().y, maps)
    }

    fn closest_map_dropping(&self, code: &str) -> Option<&MapSchema> {
        let resources = self.resources.dropping(code);
        let maps = self
            .maps
            .data
            .iter()
            .filter(|m| m.has_one_of_resource(&resources))
            .collect_vec();
        Maps::closest_from_amoung(self.data().x, self.data().y, maps)
    }

    fn closest_map_with_resource(&self, code: &str) -> Option<&MapSchema> {
        let maps = self.maps.with_ressource(code);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    fn move_to_craft(&self, code: &str) -> bool {
        if let Some(dest) = self
            .items
            .skill_to_craft(code)
            .and_then(|s| self.maps.to_craft(s))
        {
            self.action_move(dest.x, dest.y);
        }
        false
    }

    fn weapon_damage(&self) -> i32 {
        self.equipment_in(Slot::Weapon)
            .map(|w| w.total_attack_damage())
            .unwrap_or(0)
    }

    /// Equip the best gear available for the given `monster`.
    fn improve_equipment(&self, monster: &MonsterSchema) {
        self.improve_slot(Slot::Weapon, monster);
        self.improve_slot(Slot::Helmet, monster);
        self.improve_slot(Slot::LegArmor, monster);
        self.improve_slot(Slot::BodyArmor, monster);
        self.improve_slot(Slot::Boots, monster);
        self.improve_slot(Slot::Shield, monster);
        self.improve_slot(Slot::Ring1, monster);
        self.improve_slot(Slot::Ring2, monster);
        self.improve_slot(Slot::Amulet, monster);
        //self.improve_slot(Slot::Artifact1);
        //self.improve_slot(Slot::Artifact2);
        //self.improve_slot(Slot::Artifact3);
        //self.improve_slot(Slot::Consumable1);
        //self.improve_slot(Slot::Consumable2);
    }

    /// Equip the given `slot` with the best item available for the given `monster`.
    fn improve_slot(&self, slot: Slot, monster: &MonsterSchema) {
        if let Some(upgrade) = self.best_in_slot_against(slot, monster).filter(|u| {
            self.equipment_in(slot).is_none()
                || self.equipment_in(slot).is_some_and(|i| i.code != u.code)
        }) {
            info!("{}: upgrade found: {}", self.name, upgrade.code);
            let prev_equiped = self.equipment_in(slot);
            if self.amount_in_inventory(&upgrade.code) > 0 {
                let _ = self.action_equip(&upgrade.code, slot);
            } else if self.action_withdraw(&upgrade.code, 1).is_ok() {
                let _ = self.action_equip(&upgrade.code, slot);
                if let Some(i) = prev_equiped {
                    let _ = self.action_deposit(&i.code, 1);
                }
            } else {
                info!(
                    "{}: upgrade not found in bank of inventory: '{}'",
                    self.name, upgrade.code
                );
            }
        }
    }

    /// Returns the best item for the given `slot` against the given `monster`
    /// Based on item attack damage, damage increase and `monster` resistances.
    fn best_in_slot_against(&self, slot: Slot, monster: &MonsterSchema) -> Option<&ItemSchema> {
        match slot {
            Slot::Weapon => self.weapon_upgrade_in_bank(monster),
            Slot::Amulet if self.data().level >= 5 && self.data().level < 10 => {
                self.items.get("life_amulet")
            }
            Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Helmet
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet => self.armor_damage_upgrades_in_bank(slot, monster),
            Slot::Boots if self.data().level >= 20 => self.items.get("steel_boots"),
            Slot::Boots if self.data().level >= 15 => self.items.get("adventurer_boots"),
            Slot::Boots if self.data().level >= 10 => self.items.get("iron_boots"),
            Slot::Boots => self.items.get("copper_boots"),
            Slot::Shield if self.data().level >= 30 => self.items.get("golden_shield"),
            Slot::Shield if self.data().level >= 20 => self.items.get("steel_shield"),
            Slot::Shield if self.data().level >= 10 => self.items.get("slime_shield"),
            Slot::Shield => self.items.get("wooden_shield"),
            _ => None,
        }
    }

    fn has_in_bank_or_inv(&self, code: &str) -> bool {
        self.amount_in_inventory(code) > 0 || self.bank.read().is_ok_and(|b| b.has_item(code) > 0)
    }

    /// Returns
    fn weapon_upgrades(&self, monster: &MonsterSchema) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, Slot::Weapon)
            .into_iter()
            .filter(|i| {
                self.equipment_in(Slot::Weapon).is_none()
                    || self.equipment_in(Slot::Weapon).is_some_and(|e| {
                        e.attack_damage_against(monster) < i.attack_damage_against(monster)
                    })
            })
            .collect_vec()
    }

    /// Returns all best upgrade available for the given `monster` based on
    /// the currently equiped weapon and the `monster` resistances.
    fn weapon_upgrade_in_bank(&self, monster: &MonsterSchema) -> Option<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, Slot::Weapon)
            .into_iter()
            .filter(|i| self.has_in_bank_or_inv(&i.code))
            .filter(|i| {
                self.equipment_in(Slot::Weapon).is_none()
                    || self.equipment_in(Slot::Weapon).is_some_and(|e| {
                        e.attack_damage_against(monster) < i.attack_damage_against(monster)
                    })
            })
            .max_by_key(|i| i.total_attack_damage())
    }

    /// Returns the best upgrade available in bank or inventory for the given
    /// armor `slot` against the given `monster`, based on the currently equiped
    /// weapon and the `monster` resitances.
    fn armor_damage_upgrades_in_bank(
        &self,
        slot: Slot,
        monster: &MonsterSchema,
    ) -> Option<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, slot)
            .into_iter()
            .filter(|i| self.has_in_bank_or_inv(&i.code))
            .filter(|i| {
                self.damage_inc_from_slot(slot, monster) < self.damage_inc_against_with(i, monster)
            })
            .max_by_key(|i| OrderedFloat(self.damage_inc_against_with(i, monster)))
    }

    /// Returns all damage upgrades for the given armor `slot` against the given
    /// `monster`, based on the currently equiped weapon and the `monster`
    /// resitances.
    fn armor_damage_upgrades(&self, slot: Slot, monster: &MonsterSchema) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, slot)
            .into_iter()
            .filter(|i| {
                self.damage_inc_from_slot(slot, monster) < self.damage_inc_against_with(i, monster)
            })
            .collect_vec()
    }

    fn attack_damage(&self, r#type: DamageType) -> i32 {
        self.data.read().map_or(0, |d| match r#type {
            DamageType::Air => d.attack_air,
            DamageType::Earth => d.attack_earth,
            DamageType::Fire => d.attack_fire,
            DamageType::Water => d.attack_water,
        })
    }

    fn damage_increase(&self, r#type: DamageType) -> i32 {
        self.data.read().map_or(0, |d| match r#type {
            DamageType::Air => d.dmg_air,
            DamageType::Earth => d.dmg_earth,
            DamageType::Fire => d.dmg_fire,
            DamageType::Water => d.dmg_water,
        })
    }

    fn resistance(&self, r#type: DamageType) -> i32 {
        self.data.read().map_or(0, |d| match r#type {
            DamageType::Air => d.res_air,
            DamageType::Earth => d.res_earth,
            DamageType::Fire => d.res_fire,
            DamageType::Water => d.res_water,
        })
    }

    fn attack_damage_against(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                self.attack_damage(t) as f32 * (1.0 + self.damage_increase(t) as f32) / 100.0
                    * (1.0 - (monster.resistance(t) as f32))
            })
            .sum()
    }

    fn attack_damage_from(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| monster.attack_damage(t) as f32 * (1.0 - (self.resistance(t) as f32 / 100.0)))
            .sum()
    }

    fn damage_inc_against_with(&self, armor: &ItemSchema, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| {
                self.equipment_in(Slot::Weapon)
                    .map_or(1, |i| i.attack_damage(t)) as f32
                    * (1.0 + armor.damage_increase(t) as f32)
                    / 100.0
                    * (1.0 - (monster.resistance(t) as f32 / 100.0))
            })
            .sum::<f32>()
    }

    fn damage_inc_from_slot(&self, slot: Slot, monster: &MonsterSchema) -> f32 {
        self.equipment_in(slot)
            .map_or(0.0, |i| self.damage_inc_against_with(i, monster))
    }

    // fn fight_until_unsuccessful(&self, x: i32, y: i32) {
    //     let _ = self.move_to(x, y);

    //     loop {
    //         if let Err(Error::ResponseError(res)) = self.fight() {
    //             if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
    //                 error!("{}: needs to cooldown", self.name);
    //                 self.cool_down(self.remaining_cooldown());
    //             }
    //             if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
    //                 error!("{}: inventory is full", self.name);
    //                 self.move_to_bank();
    //                 self.deposit_all();
    //                 let _ = self.move_to(x, y);
    //             }
    //         }
    //     }
    // }
}

#[derive(Debug, Default, PartialEq, Clone, Deserialize)]
pub enum Role {
    Fighter,
    Miner,
    Woodcutter,
    Fisher,
    Weaponcrafter,
    #[default]
    Idle,
}

impl Role {
    pub fn to_skill(&self) -> Option<Skill> {
        match *self {
            Role::Fighter => None,
            Role::Miner => Some(Skill::Mining),
            Role::Woodcutter => Some(Skill::Woodcutting),
            Role::Fisher => Some(Skill::Fishing),
            Role::Weaponcrafter => Some(Skill::Weaponcrafting),
            Role::Idle => None,
        }
    }
}

pub enum Action {
    Fight,
    Gather,
    Craft,
    Withdraw,
    Deposit,
}

pub struct Order {}
