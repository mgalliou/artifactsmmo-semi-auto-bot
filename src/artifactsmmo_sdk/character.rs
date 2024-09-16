use super::{
    account::Account,
    api::{events::EventsApi, my_character::MyCharacterApi},
    bank::Bank,
    char_config::CharConfig,
    compute_damage,
    equipment::Equipment,
    items::{DamageType, Items, Slot, Type},
    maps::Maps,
    monsters::Monsters,
    resources::Resources,
    skill::Skill,
    ItemSchemaExt, MonsterSchemaExt,
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
    events_api: EventsApi,
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
            events_api: EventsApi::new(
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
            .name(
                char.data
                    .read()
                    .map_or("unknown".to_string(), |d| d.name.to_owned()),
            )
            .spawn(move || {
                char.run2();
            })
    }

    fn run2(&self) {
        info!("{}: started !", self.name);
        if self.role() != Role::Fighter
            && self
                .equipment_in(Slot::Weapon)
                .is_some_and(|w| w.code == "wooden_stick")
        {
            let _ = self.action_unequip(Slot::Weapon);
            self.deposit_all();
        };
        while self.role() != Role::Idle {
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
            if self.role() == Role::Fighter {
                if let Some((map, equipment)) = self.best_monster_map_with_equipment() {
                    self.equip_equipment(&equipment);
                    self.action_move(map.x, map.y);
                    let _ = self.action_fight();
                }
            } else if self.role() == Role::Miner
                || self.role() == Role::Woodcutter
                || self.role() == Role::Fisher
            {
                if let Some(map) = self.best_resource_map() {
                    self.action_move(map.x, map.y);
                    let _ = self.action_gather();
                }
            }
        }
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }

    fn role(&self) -> Role {
        self.conf.read().map_or(Role::Idle, |d| d.role)
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
        if self.task().is_empty() || self.task_finished() {
            if self.task_finished() {
                let _ = self.action_complete_task();
            }
            let _ = self.action_accept_task();
        }
    }

    fn task(&self) -> String {
        self.data
            .read()
            .map_or("".to_string(), |d| d.task.to_owned())
    }

    fn task_type(&self) -> String {
        self.data
            .read()
            .map_or("".to_string(), |d| d.task.to_owned())
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

    /// Move the `Character` to the closest map containing the `code` resource,
    /// then fight. Returns true is the API request went successfully.
    fn kill_monster(&self, code: &str) -> bool {
        if let Some(map) = self.closest_map_with_content(code) {
            return self.action_move(map.x, map.y) && self.action_fight().is_ok();
        }
        false
    }

    /// Checks if the `Character` could kill the given `monster` with the given
    /// `equipment`
    fn can_kill_with(&self, monster: &MonsterSchema, equipment: &Equipment) -> bool {
        let turns_to_kill = (monster.hp as f32 / equipment.attack_damage_against(monster)).ceil();
        let turns_to_be_killed = ((self.base_health() + equipment.health_increase()) as f32
            / equipment.attack_damage_from(monster))
        .ceil();
        debug!(
            "{}: '{}': turn to kill: {}, turns to be killed {}",
            self.name, monster.code, turns_to_kill, turns_to_be_killed
        );
        turns_to_kill <= turns_to_be_killed
    }

    fn level(&self) -> i32 {
        self.data.read().map_or(1, |d| d.level)
    }
    /// Returns the base health of the `Character` without its equipment.
    fn base_health(&self) -> i32 {
        115 + 5 * self.level()
    }

    /// Move the `Character` to the closest map containing the `code` resource,
    /// then gather. Returns true is the API request went successfully.
    fn gather_resource(&self, code: &str) -> bool {
        if let Some(map) = self.closest_map_with_content(code) {
            return self.action_move(map.x, map.y) && self.action_gather().is_ok();
        }
        false
    }

    /// Returns the next skill that should leveled by the Character, based on
    /// its configuration and the items available in bank.
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

    /// Returns the map containing the best monster for the current character
    /// alongside the best equipment available to fight the target `monster` if
    /// it call be killed with it. The monster priority order is events,
    /// then tasks, then target from config file, then lowest level target.
    fn best_monster_map_with_equipment(&self) -> Option<(MapSchema, Equipment)> {
        if let Ok(events) = self.events_api.all() {
            for event in events {
                if let Some(monster) = event
                    .map
                    .content
                    .as_ref()
                    .and_then(|c| self.monsters.get(&c.code))
                {
                    let equipment = self.best_available_equipment_against(monster);
                    if self.can_kill_with(monster, &equipment) {
                        return Some(((*event.map.clone()), equipment));
                    }
                }
            }
        }
        if self.conf().do_tasks && self.task_type() == "monsters" && !self.task_finished() {
            if let Some(monster) = self.monsters.get(&self.task()) {
                let equipment = self.best_available_equipment_against(monster);
                if self.can_kill_with(monster, &equipment) {
                    return Some((
                        self.closest_map_with_content(&monster.code)?.clone(),
                        equipment,
                    ));
                }
            }
        }
        if let Some(monster_code) = &self.conf().fight_target {
            if let Some(monster) = self.monsters.get(monster_code) {
                let equipment = self.best_available_equipment_against(monster);
                if self.can_kill_with(monster, &equipment) {
                    return Some((
                        self.closest_map_with_content(&monster.code)?.clone(),
                        equipment,
                    ));
                }
            }
        } 
        if let Some(monster) = self.monsters.lowest_providing_exp(self.level()) {
            let equipment = self.best_available_equipment_against(monster);
            if self.can_kill_with(monster, &equipment) {
                return Some((
                    self.closest_map_with_content(&monster.code)?.clone(),
                    equipment,
                ));
            }
        }
        None
    }

    fn best_resource_map(&self) -> Option<MapSchema> {
        if let Ok(events) = self.events_api.all() {
            for event in events {
                if let Some(resource) = event
                    .map
                    .content
                    .as_ref()
                    .and_then(|c| self.resources.get(&c.code))
                {
                    if self.can_gather(resource) {
                        return Some(*event.map.clone());
                    }
                }
            }
        }
        if let Some(item) = self.conf().target_item {
            if let Some(resource) = self
                .resources
                .dropping(&item)
                .iter()
                .find(|r| self.can_gather(r))
            {
                return self.closest_map_with_content(&resource.code).cloned();
            }
            warn!(
                "{}: does not have required level to gather '{}'.",
                self.name, item
            );
        }
        if let Some(skill) = self.role().to_skill() {
            if let Some(resource) = self
                .resources
                .lowest_providing_exp(self.skill_level(skill), skill)
            {
                return self.closest_map_with_content(&resource.code).cloned();
            }
        }
        None
    }

    fn task_finished(&self) -> bool {
        self.data.read().map_or(0, |d| d.task_progress)
            >= self.data.read().map_or(0, |d| d.task_total)
    }

    fn equipment_in(&self, slot: Slot) -> Option<&ItemSchema> {
        self.data
            .read()
            .map(|d| match slot {
                Slot::Weapon => d.weapon_slot.to_owned(),
                Slot::Shield => d.shield_slot.to_owned(),
                Slot::Helmet => d.helmet_slot.to_owned(),
                Slot::BodyArmor => d.body_armor_slot.to_owned(),
                Slot::LegArmor => d.leg_armor_slot.to_owned(),
                Slot::Boots => d.boots_slot.to_owned(),
                Slot::Ring1 => d.ring1_slot.to_owned(),
                Slot::Ring2 => d.ring2_slot.to_owned(),
                Slot::Amulet => d.amulet_slot.to_owned(),
                Slot::Artifact1 => d.artifact1_slot.to_owned(),
                Slot::Artifact2 => d.artifact2_slot.to_owned(),
                Slot::Artifact3 => d.artifact3_slot.to_owned(),
                Slot::Consumable1 => d.consumable1_slot.to_owned(),
                Slot::Consumable2 => d.consumable2_slot.to_owned(),
            })
            .map(|i| self.items.get(&i))
            .ok()?
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
        self.data.read().map_or(1, |d| match skill {
            Skill::Cooking => d.cooking_level,
            Skill::Fishing => d.fishing_level,
            Skill::Gearcrafting => d.gearcrafting_level,
            Skill::Jewelrycrafting => d.jewelrycrafting_level,
            Skill::Mining => d.mining_level,
            Skill::Weaponcrafting => d.weaponcrafting_level,
            Skill::Woodcutting => d.woodcutting_level,
        })
    }

    /// Returns a copy of the inventory to be used while depositing or
    /// withdrawing items.
    fn inventory_copy(&self) -> Vec<InventorySlot> {
        self.data
            .read()
            .map(|d| d.inventory.iter().flatten().cloned().collect_vec())
            .into_iter()
            .flatten()
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
        let amount = self.has_in_inventory(code);
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
        self.data
            .read()
            .map(|d| {
                d.cooldown_expiration
                    .as_ref()
                    .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
            })
            .ok()?
    }

    fn inventory_is_full(&self) -> bool {
        self.inventory_total() >= self.data.read().map_or(100, |d| d.inventory_max_items)
            || self
                .data
                .read()
                .is_ok_and(|d| d.inventory.iter().flatten().all(|s| s.quantity > 0))
    }

    fn has_in_inventory(&self, code: &str) -> i32 {
        self.data.read().map_or(0, |d| {
            d.inventory
                .iter()
                .flatten()
                .find(|i| i.code == code)
                .map_or(0, |i| i.quantity)
        })
    }

    fn inventory_free_space(&self) -> i32 {
        self.data
            .read()
            .map_or(0, |d| d.inventory_max_items - self.inventory_total())
    }

    fn inventory_total(&self) -> i32 {
        self.data.read().map_or(0, |d| {
            d.inventory.iter().flatten().map(|i| i.quantity).sum()
        })
    }

    fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats(code)
            .iter()
            .filter(|mat| mat.quantity > 0)
            .map(|mat| self.has_in_inventory(&mat.code) / mat.quantity)
            .min()
            .unwrap_or(0)
    }

    fn closest_map_among<'a>(&'a self, maps: Vec<&'a MapSchema>) -> Option<&MapSchema> {
        let (x, y) = self.position();
        Maps::closest_from_amoung(x, y, maps)
    }

    fn position(&self) -> (i32, i32) {
        let (x, y) = self.data.read().map_or((0, 0), |d| (d.x, d.y));
        (x, y)
    }

    fn closest_map_with_content(&self, code: &str) -> Option<&MapSchema> {
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

    fn equip_equipment(&self, equipment: &Equipment) {
        Slot::iter().for_each(|s| {
            let prev_equiped = self.equipment_in(s);
            if let Some(item) = equipment.slot(s) {
                if prev_equiped.is_some_and(|e| e.code == item.code) {
                    debug!("{}: item already equiped: '{}'.", self.name, item.code)
                } else if self.has_in_inventory(&item.code) > 0 {
                    let _ = self.action_equip(&item.code, s);
                } else if self.has_in_bank(&item.code) > 0
                    && self.action_withdraw(&item.code, 1).is_ok()
                {
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
        let weapons = self.best_available_weapon_against(monster);
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
            Slot::Amulet if self.level() >= 5 && self.level() < 10 => self.items.get("life_amulet"),
            Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Helmet
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet => self.best_available_armor_against_with_weapon(slot, monster, weapon),
            Slot::Boots if self.level() >= 20 && self.has_available("steel_boots", slot) => {
                self.items.get("steel_boots")
            }
            Slot::Boots if self.level() >= 15 && self.has_available("adventurer_boots", slot) => {
                self.items.get("adventurer_boots")
            }
            Slot::Boots if self.level() >= 10 && self.has_available("iron_boots", slot) => {
                self.items.get("iron_boots")
            }
            Slot::Boots if self.has_available("copper_boots", slot) => {
                self.items.get("copper_boots")
            }
            Slot::Shield if self.level() >= 30 && self.has_available("golden_shield", slot) => {
                self.items.get("golden_shield")
            }
            Slot::Shield if self.level() >= 20 && self.has_available("steel_shield", slot) => {
                self.items.get("steel_shield")
            }
            Slot::Shield if self.level() >= 10 && self.has_available("slime_shield", slot) => {
                self.items.get("slime_shield")
            }
            Slot::Shield if self.has_available("wooden_shield", slot) => {
                self.items.get("wooden_shield")
            }
            _ => None,
        }
    }

    fn has_in_bank(&self, code: &str) -> i32 {
        self.bank.read().map_or(0, |b| b.has_item(code))
    }
    fn has_in_bank_or_inv(&self, code: &str) -> bool {
        self.has_in_bank(code) > 0 || self.has_in_inventory(code) > 0
    }

    fn has_available(&self, code: &str, slot: Slot) -> bool {
        self.has_in_bank_or_inv(code) || self.equipment_in(slot).is_some_and(|e| e.code == code)
    }

    fn has_equiped(&self, code: &str) -> bool {
        Slot::iter().any(|s| self.equipment_in(s).is_some_and(|e| e.code == code))
    }

    /// Returns all the best weapon upgrades available for the given `monster` based on
    /// the currently equiped weapon and the `monster` resistances.
    fn best_available_weapon_against(&self, monster: &MonsterSchema) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.level(), Slot::Weapon)
            .into_iter()
            .filter(|i| self.has_available(&i.code, Slot::Weapon))
            .max_set_by_key(|i| OrderedFloat(i.attack_damage_against(monster)))
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
            .equipable_at_level(self.level(), slot)
            .into_iter()
            .filter(|i| self.has_available(&i.code, slot))
            .max_by_key(|i| {
                OrderedFloat(self.armor_attack_damage_against_with_weapon(i, monster, weapon))
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

    fn attack_damage_from(&self, monster: &MonsterSchema) -> f32 {
        DamageType::iter()
            .map(|t| compute_damage(monster.attack_damage(t), 0, self.resistance(t)))
            .sum()
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

    fn can_gather(&self, resource: &ResourceSchema) -> bool {
        self.skill_level(resource.skill.into()) >= resource.level
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

#[derive(Debug, Default, PartialEq, Copy, Clone, Deserialize)]
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
