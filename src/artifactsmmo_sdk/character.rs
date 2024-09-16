use super::{
    account::Account,
    api::my_character::MyCharacterApi,
    bank::Bank,
    char_config::CharConfig,
    compute_damage,
    equipment::Equipment,
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
                if self.levelup_by_crafting(skill) {
                    continue;
                }
            }
            if let Some(craft) = self.conf().target_craft {
                if self.craft_all_from_bank(&craft) > 0 {
                    continue;
                }
            }
            if let Some(monster) = self.target_monster() {
                let equipment = self.best_available_equipment_against(monster);
                self.equip_equipment(&equipment);
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

    fn equipment(&self) -> Equipment {
        self.data
            .read()
            .map_or(Equipment::default(), |d| Equipment {
                weapon: self.items.get(&d.weapon_slot),
                shield: self.items.get(&d.shield_slot),
                helmet: self.items.get(&d.helmet_slot),
                body_armor: self.items.get(&d.boots_slot),
                leg_armor: self.items.get(&d.leg_armor_slot),
                boots: self.items.get(&d.boots_slot),
                ring1: self.items.get(&d.ring1_slot),
                ring2: self.items.get(&d.ring2_slot),
                amulet: self.items.get(&d.amulet_slot),
                artifact1: self.items.get(&d.artifact1_slot),
                artifact2: self.items.get(&d.artifact2_slot),
                artifact3: self.items.get(&d.artifact3_slot),
                consumable1: self.items.get(&d.consumable1_slot),
                consumable2: self.items.get(&d.consumable2_slot),
            })
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

    fn equip_equipment(&self, equipment: &Equipment) {
        Slot::iter().for_each(|s| {
            let prev_equiped = self.equipment_in(s);
            if let Some(item) = equipment.slot(s) {
                if prev_equiped.is_some_and(|e| e.code == item.code) {
                    debug!("{}: item already equiped: '{}'.", self.name, item.code)
                } else if self.amount_in_inventory(&item.code) > 0 {
                    let _ = self.action_equip(&item.code, s);
                } else if self.bank.read().is_ok_and(|b| {
                    b.has_item(&item.code) > 0 && self.action_withdraw(&item.code, 1).is_ok()
                }) {
                    let _ = self.action_equip(&item.code, s);
                    if let Some(i) = prev_equiped {
                        let _ = self.action_deposit(&i.code, 1);
                    }
                } else {
                    info!(
                        "{}: upgrade not found in bank of inventory: '{}'",
                        self.name, item.code
                    );
                }
            }
        })
    }

    fn best_available_equipment_against(&self, monster: &MonsterSchema) -> Equipment {
        let mut weapons = self.best_available_weapon_against(monster);
        if weapons.is_empty() {
            weapons.push(self.equipment_in(Slot::Weapon).expect("should exit"))
        }
        let best_equipment = weapons
            .iter()
            .map(|w| self.best_available_equipment_against_with_weapon(monster, w))
            .max_by_key(|e| OrderedFloat(e.attack_damage_against(monster)));
        if let Some(best_equipment) = best_equipment {
            return best_equipment;
        }
        self.equipment()
    }

    fn best_available_equipment_against_with_weapon<'a>(
        &'a self,
        monster: &MonsterSchema,
        weapon: &'a ItemSchema,
    ) -> Equipment {
        Equipment {
            weapon: Some(weapon),
            shield: self.best_in_slot_available_against_with_weapon(Slot::Shield, monster, weapon),
            helmet: self.best_in_slot_available_against_with_weapon(Slot::Helmet, monster, weapon),
            body_armor: self.best_in_slot_available_against_with_weapon(
                Slot::BodyArmor,
                monster,
                weapon,
            ),
            leg_armor: self.best_in_slot_available_against_with_weapon(
                Slot::LegArmor,
                monster,
                weapon,
            ),
            boots: self.best_in_slot_available_against_with_weapon(Slot::Boots, monster, weapon),
            ring1: self.best_in_slot_available_against_with_weapon(Slot::Ring1, monster, weapon),
            ring2: self.best_in_slot_available_against_with_weapon(Slot::Ring2, monster, weapon),
            amulet: self.best_in_slot_available_against_with_weapon(Slot::Amulet, monster, weapon),
            artifact1: self.best_in_slot_available_against_with_weapon(
                Slot::Artifact1,
                monster,
                weapon,
            ),
            artifact2: self.best_in_slot_available_against_with_weapon(
                Slot::Artifact2,
                monster,
                weapon,
            ),
            artifact3: self.best_in_slot_available_against_with_weapon(
                Slot::Artifact3,
                monster,
                weapon,
            ),
            consumable1: self.best_in_slot_available_against_with_weapon(
                Slot::Consumable1,
                monster,
                weapon,
            ),
            consumable2: self.best_in_slot_available_against_with_weapon(
                Slot::Consumable2,
                monster,
                weapon,
            ),
        }
    }

    /// Equip the given `slot` with the best item available for the given `monster`.
    fn improve_slot(&self, slot: Slot, monster: &MonsterSchema) {
        if let Some(upgrade) = self
            .best_in_slot_available_against(slot, monster)
            .filter(|u| {
                self.equipment_in(slot).is_none()
                    || self.equipment_in(slot).is_some_and(|i| i.code != u.code)
            })
        {
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

    /// Returns the best item available for the given `slot` against the given
    /// `monster`, based on item attack damage, damage increase and `monster`
    /// resistances.
    fn best_in_slot_available_against(
        &self,
        slot: Slot,
        monster: &MonsterSchema,
    ) -> Option<&ItemSchema> {
        match slot {
            Slot::Weapon => self.best_weapon_available(monster),
            Slot::Amulet if self.data().level >= 5 && self.data().level < 10 => {
                self.items.get("life_amulet")
            }
            Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Helmet
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet => self.armor_damage_upgrade_in_bank(slot, monster),
            Slot::Boots if self.data().level >= 20 && self.has_in_bank_or_inv("steel_boots") => {
                self.items.get("steel_boots")
            }
            Slot::Boots
                if self.data().level >= 15 && self.has_in_bank_or_inv("adventurer_boots") =>
            {
                self.items.get("adventurer_boots")
            }
            Slot::Boots if self.data().level >= 10 && self.has_in_bank_or_inv("iron_boots") => {
                self.items.get("iron_boots")
            }
            Slot::Boots if self.has_in_bank_or_inv("copper_boots") => {
                self.items.get("copper_boots")
            }
            Slot::Shield if self.data().level >= 30 && self.has_in_bank_or_inv("golden_shield") => {
                self.items.get("golden_shield")
            }
            Slot::Shield if self.data().level >= 20 && self.has_in_bank_or_inv("steel_shield") => {
                self.items.get("steel_shield")
            }
            Slot::Shield if self.data().level >= 10 && self.has_in_bank_or_inv("slime_shield") => {
                self.items.get("slime_shield")
            }
            Slot::Shield if self.has_in_bank_or_inv("wooden_shield") => {
                self.items.get("wooden_shield")
            }
            _ => None,
        }
    }

    /// Returns the best item available for the given `slot` against the given
    /// `monster`, based on item attack damage, damage increase and `monster`
    /// resistances.
    fn best_in_slot_available_against_with_weapon(
        &self,
        slot: Slot,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
    ) -> Option<&ItemSchema> {
        match slot {
            Slot::Amulet if self.data().level >= 5 && self.data().level < 10 => {
                self.items.get("life_amulet")
            }
            Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Helmet
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet => self.best_available_armor_against_with_weapon(slot, monster, weapon),
            Slot::Boots if self.data().level >= 20 && self.has_available("steel_boots", slot) => {
                self.items.get("steel_boots")
            }
            Slot::Boots if self.data().level >= 15 && self.has_available("adventurer_boots", slot) => {
                self.items.get("adventurer_boots")
            }
            Slot::Boots if self.data().level >= 10 && self.has_available("iron_boots", slot) => {
                self.items.get("iron_boots")
            }
            Slot::Boots if self.has_available("copper_boots", slot) => self.items.get("copper_boots"),
            Slot::Shield if self.data().level >= 30 && self.has_available("golden_shield", slot) => {
                self.items.get("golden_shield")
            }
            Slot::Shield if self.data().level >= 20 && self.has_available("steel_shield", slot) => {
                self.items.get("steel_shield")
            }
            Slot::Shield if self.data().level >= 10 && self.has_available("slime_shield", slot) => {
                self.items.get("slime_shield")
            }
            Slot::Shield if self.has_available("wooden_shield", slot) => self.items.get("wooden_shield"),
            _ => None,
        }
    }

    fn has_in_bank_or_inv(&self, code: &str) -> bool {
        self.amount_in_inventory(code) > 0 || self.bank.read().is_ok_and(|b| b.has_item(code) > 0)
    }

    fn has_available(&self, code: &str, slot: Slot) -> bool {
        self.has_in_bank_or_inv(code) || self.equipment_in(slot).is_some_and(|e| e.code == code)
    }

    fn has_equiped(&self, code: &str) -> bool {
        Slot::iter().any(|s| self.equipment_in(s).is_some_and(|e| e.code == code))
    }

    /// Returns
    fn best_weapons_against(&self, monster: &MonsterSchema) -> Vec<&ItemSchema> {
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

    /// Returns all the best weapon upgrades available for the given `monster` based on
    /// the currently equiped weapon and the `monster` resistances.
    fn best_available_weapon_against(&self, monster: &MonsterSchema) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, Slot::Weapon)
            .into_iter()
            .filter(|i| self.has_available(&i.code, Slot::Weapon))
            .max_set_by_key(|i| OrderedFloat(i.attack_damage_against(monster)))
    }

    /// Returns the best weapon upgrade available for the given `monster` based on
    /// the currently equiped weapon and the `monster` resistances.
    fn best_weapon_available(&self, monster: &MonsterSchema) -> Option<&ItemSchema> {
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
    fn armor_damage_upgrade_in_bank(
        &self,
        slot: Slot,
        monster: &MonsterSchema,
    ) -> Option<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, slot)
            .into_iter()
            .filter(|i| self.has_in_bank_or_inv(&i.code))
            .filter(|i| {
                self.slot_attack_damage_against(slot, monster)
                    < self.armor_attack_damage_against(i, monster)
            })
            .max_by_key(|i| OrderedFloat(self.armor_attack_damage_against(i, monster)))
    }

    /// Returns the best upgrade available in bank or inventory for the given
    /// armor `slot` against the given `monster`, based on the currently equiped
    /// weapon and the `monster` resitances.
    fn best_available_armor_against_with_weapon(
        &self,
        slot: Slot,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
    ) -> Option<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, slot)
            .into_iter()
            .filter(|i| self.has_available(&i.code, slot))
            .max_by_key(|i| {
                OrderedFloat(self.armor_attack_damage_against_with_weapon(i, monster, weapon))
            })
    }

    /// Returns all damage upgrades for the given armor `slot` against the given
    /// `monster`, based on the currently equiped weapon and the `monster`
    /// resitances.
    fn best_armors_against(&self, slot: Slot, monster: &MonsterSchema) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.data().level, slot)
            .into_iter()
            .filter(|i| {
                self.slot_attack_damage_against(slot, monster)
                    < self.armor_attack_damage_against(i, monster)
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

    /// Takes an `armor` and returns the total attack damage it provides
    /// combined with the weapon currenly equiped by the `Character` against
    /// the given `monster
    // TODO: check `armor` is an armor `ItemSchema`
    fn armor_attack_damage_against(&self, armor: &ItemSchema, monster: &MonsterSchema) -> f32 {
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

    fn armor_attack_damage_against_with_weapon(
        &self,
        armor: &ItemSchema,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
    ) -> f32 {
        DamageType::iter()
            .map(|t| {
                compute_damage(
                    weapon.attack_damage(t),
                    armor.damage_increase(t),
                    monster.resistance(t),
                )
            })
            .sum::<f32>()
    }

    /// Takes a `slot` and returns the total attack damage it provides
    /// combined with the weapon currently equiped by the `Character` against the
    /// given `monster`
    fn slot_attack_damage_against(&self, slot: Slot, monster: &MonsterSchema) -> f32 {
        self.equipment_in(slot)
            .map_or(0.0, |i| self.armor_attack_damage_against(i, monster))
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
