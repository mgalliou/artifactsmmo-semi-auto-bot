use super::{
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    bank::Bank,
    char_config::CharConfig,
    compute_damage,
    config::Config,
    equipment::Equipment,
    events::Events,
    game::Game,
    items::{DamageType, ItemSource, Items, Slot, Type},
    maps::Maps,
    monsters::Monsters,
    orderboard::{Order, OrderBoard},
    resources::Resources,
    skill::Skill,
    ActiveEventSchemaExt, FightSchemaExt, ItemSchemaExt, MonsterSchemaExt, SkillSchemaExt,
};
use actions::{FightError, PostCraftAction, SkillError};
use artifactsmmo_openapi::models::{
    CharacterSchema, FightSchema, InventorySlot, ItemSchema, MapContentSchema, MapSchema,
    MonsterSchema, ResourceSchema, SkillDataSchema,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    cmp::{min, Ordering},
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
    pub name: String,
    my_api: MyCharacterApi,
    api: CharactersApi,
    game: Arc<Game>,
    maps: Arc<Maps>,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    items: Arc<Items>,
    events: Arc<Events>,
    bank: Arc<Bank>,
    orderboard: Arc<OrderBoard>,
    pub conf: Arc<RwLock<CharConfig>>,
    pub data: Arc<RwLock<CharacterSchema>>,
}

impl Character {
    pub fn new(
        config: &Config,
        game: &Arc<Game>,
        bank: &Arc<Bank>,
        conf: &Arc<RwLock<CharConfig>>,
        data: &Arc<RwLock<CharacterSchema>>,
    ) -> Character {
        Character {
            name: data.read().map(|d| d.name.to_owned()).unwrap(),
            conf: conf.clone(),
            my_api: MyCharacterApi::new(&config.base_url, &config.token),
            api: CharactersApi::new(&config.base_url, &config.token),
            game: game.clone(),
            maps: game.maps.clone(),
            resources: game.resources.clone(),
            monsters: game.monsters.clone(),
            items: game.items.clone(),
            events: game.events.clone(),
            orderboard: game.billboard.clone(),
            bank: bank.clone(),
            data: data.clone(),
        }
    }

    pub fn run(char: Arc<Character>) -> Result<JoinHandle<()>, io::Error> {
        thread::Builder::new()
            .name(char.name.to_owned())
            .spawn(move || {
                char.run_loop();
            })
    }

    fn run_loop(&self) {
        info!("{}: started !", self.name);
        self.handle_wooden_stick();
        loop {
            if self.conf.read().unwrap().idle {
                continue;
            }
            self.events.refresh();
            self.process_inventory();
            if self.levelup_skills() {
                continue;
            }
            if self.conf().do_events && self.handle_events() {
                continue;
            }
            if self.handle_orderboard() {
                continue;
            }
            if let Some(craft) = self.conf().target_craft {
                if self
                    .craft_max_from_bank(&craft, PostCraftAction::Deposit)
                    .is_ok()
                {
                    continue;
                }
            }
            if self.conf().do_tasks && self.handle_task() {
                continue;
            }
            if self.role() == Role::Fighter && self.find_and_kill() {
                continue;
            }
            if self.is_gatherer() && self.find_and_gather() {
                continue;
            }
            info!("{}: no action found, sleeping for 30sec.", self.name);
            sleep(Duration::from_secs(30));
        }
    }

    fn handle_wooden_stick(&self) {
        if self.role() != Role::Fighter && self.has_equiped("wooden_stick") > 0 {
            let _ = self.action_unequip(Slot::Weapon, 1);
            let _ = self.action_deposit("wooden_stick", 1);
        };
    }

    /// If inventory is full, process the raw materials if possible and deposit
    /// all the consumables and resources in inventory to the bank.
    fn process_inventory(&self) {
        if self.inventory_is_full() {
            if self.conf().process_gathered {
                self.process_raw_mats();
            }
            self.deposit_all()
        }
    }

    /// Returns the next skill that should leveled by the Character, based on
    /// its configuration and the items available in bank.
    fn levelup_skills(&self) -> bool {
        let mut skills = vec![];
        if self.conf().gearcraft {
            skills.push(Skill::Gearcrafting);
        }
        if self.conf().weaponcraft {
            skills.push(Skill::Weaponcrafting);
        }
        if self.conf().jewelcraft {
            skills.push(Skill::Jewelrycrafting);
        }
        if self.conf().cook {
            skills.push(Skill::Cooking);
        }
        skills.sort_by_key(|s| self.skill_level(*s));
        skills.into_iter().any(|skill| self.level_skill(skill))
    }

    fn level_skill(&self, skill: Skill) -> bool {
        self.items
            .best_for_leveling(self.skill_level(skill), skill)
            .iter()
            .min_by_key(|i| {
                self.bank
                    .missing_mats_quantity(&i.code, self.max_craftable_items(&i.code))
            })
            .is_some_and(|i| {
                info!("{} trying to craft to level {}", self.name, i.code);
                match self.craft_from_bank(
                    &i.code,
                    self.max_craftable_items(&i.code),
                    PostCraftAction::Recycle,
                ) {
                    Ok(_) => {
                        self.deposit_all();
                        true
                    }
                    Err(e) => {
                        if let SkillError::InsuffisientMaterials = e {
                            self.bank
                                .missing_mats_for(&i.code, self.max_craftable_items(&i.code))
                                .iter()
                                .for_each(|m| {
                                    self.orderboard.order_item(&self.name, &m.code, m.quantity)
                                })
                        }
                        false
                    }
                }
            })
    }

    /// Finds the best item  to level the given `skill` and crafts the
    /// maximum amount that can be crafted in one go with the material
    /// availables in bank. Items are crafted then recycled until no more items
    /// can be crafted or until crafting no longer provides XP.
    //TODO: handle item already in inventory
    fn levelup_by_crafting(&self, skill: Skill) -> bool {
        let mut crafted_once = false;
        if let Some(best) = self
            .items
            .best_for_leveling_hc(self.skill_level(skill), skill)
        {
            info!("{}: leveling {:#?} by crafting.", self.name, skill);
            self.deposit_all();
            self.withdraw_max_mats_for(&best.code);
            let mut crafted = -1;
            while self.skill_level(skill) - best.level <= 10 && crafted != 0 {
                crafted_once = true;
                // TODO ge prices handling
                crafted = self.craft_max_from_inventory(&best.code);
                if crafted > 0 {
                    let _ = self.action_recycle(&best.code, crafted);
                }
            }
            self.deposit_all_of_type(Type::Resource);
        }
        crafted_once
    }

    fn handle_orderboard(&self) -> bool {
        self.orderboard
            .orders()
            .iter()
            .cloned()
            .any(|r| self.handle_order(r))
    }

    fn handle_order(&self, order: Arc<Order>) -> bool {
        if order.complete() && !order.turned_in() {
            let n = self.has_in_inventory(&order.item);
            if n >= order.missing() {
                self.deposit_all();
                order.inc_deposited(n);
            }
            if order.turned_in() {
                self.orderboard.remove_order(&order);
            }
            true
        } else if let Some(progress) = self.progress_order(&order) {
            order.inc_progress(progress);
            info!(
                "{} progressed by {} on order: {}.",
                self.name, progress, order
            );
            true
        } else {
            false
        }
    }

    fn progress_order(&self, order: &Order) -> Option<i32> {
        self.items
            .source_of(&order.item)
            .iter()
            .find_map(|s| match s {
                ItemSource::Resource(r) => self
                    .gather_resource(r, None)
                    .ok()
                    .map(|gather| gather.amount_of(&order.item)),
                ItemSource::Monster(m) => self
                    .kill_monster(m, None)
                    .ok()
                    .map(|fight| fight.amount_of(&order.item)),
                ItemSource::Craft => {
                    let quantity = min(
                        self.max_craftable_items_from_bank(&order.item),
                        order.quantity,
                    );
                    // TODO: lock number of item being crafted
                    match self.craft_from_bank(&order.item, quantity, PostCraftAction::None) {
                        Ok(i) => Some(i),
                        Err(e) => {
                            if let SkillError::InsuffisientMaterials = e {
                                self.bank
                                    .missing_mats_for(&order.item, order.quantity)
                                    .iter()
                                    .for_each(|m| {
                                        self.orderboard.order_item(&self.name, &m.code, m.quantity)
                                    })
                            }
                            None
                        }
                    }
                }
                ItemSource::Task => None,
            })
    }

    fn handle_events(&self) -> bool {
        if self.role() == Role::Fighter {
            if self.handle_monster_event() {
                return true;
            }
            if self.handle_resource_event() {
                return true;
            }
        } else {
            if self.handle_resource_event() {
                return true;
            }
            if self.handle_monster_event() {
                return true;
            }
        }
        false
    }

    fn handle_task(&self) -> bool {
        if self.task().is_empty() || self.task_finished() {
            if self.task_finished() {
                let _ = self.action_complete_task();
            }
            if self.role() == Role::Fighter {
                let _ = self.action_accept_task("monsters");
            } else {
                let _ = self.action_accept_task("items");
            }
        }
        if let Some(monster) = self.monsters.get(&self.task()) {
            if self.kill_monster(monster, None).is_ok() {
                return true;
            }
        }
        if self.handle_item_task() {
            return true;
        }
        false
    }

    fn handle_resource_event(&self) -> bool {
        for event in self.events.of_type("resource") {
            if let Some(resource) = self.resources.get(event.content_code()) {
                return self.gather_resource(resource, Some(&event.map)).is_ok();
            }
        }
        false
    }

    fn handle_monster_event(&self) -> bool {
        self.events.of_type("monster").iter().any(|e| {
            self.monsters
                .get(e.content_code())
                .is_some_and(|m| self.kill_monster(m, Some(&e.map)).is_ok())
        })
    }

    fn handle_item_task(&self) -> bool {
        let in_bank = self.bank.has_item(&self.task());
        let item = &self.task();
        let missing = self.task_missing();
        let craftable = self.bank.has_mats_for(item);

        if in_bank >= missing {
            self.deposit_all();
            if missing > self.inventory_free_space() {
                self.action_withdraw(item, self.inventory_free_space())
            } else {
                self.action_withdraw(item, missing)
            };
            return self.action_task_trade(item, self.has_in_inventory(&self.task()));
        } else if self.can_craft(item) && craftable > 0 {
            return self
                .craft_from_bank(
                    item,
                    min(
                        self.max_craftable_items_from_bank(item),
                        self.task_missing(),
                    ),
                    PostCraftAction::Deposit,
                )
                .is_ok_and(|n| n > 0);
        } else {
            error!(
                "{}: missing item in bank to complete task: '{}' {}/{}",
                self.name,
                self.task(),
                self.task_progress(),
                self.task_total()
            )
        }
        false
    }

    /// Process the raw materials in the Character inventory by converting the
    /// materials having only one possible receipe.
    fn process_raw_mats(&self) {
        self.inventory_raw_mats()
            .into_iter()
            .filter_map(|rm| self.items.unique_craft(&rm.code))
            .filter(|cw| self.has_mats_for(&cw.code) > 0)
            .for_each(|p| {
                self.craft_max_from_inventory(&p.code);
            });
    }

    /// Find a target and kill it if possible.
    fn find_and_kill(&self) -> bool {
        if let Some(monster_code) = &self.conf().fight_target {
            if let Some(monster) = self.monsters.get(monster_code) {
                if self.kill_monster(monster, None).is_ok() {
                    return true;
                }
            }
        }
        // TODO: find highest killable
        if let Some(monster) = self.monsters.highest_providing_exp(self.level()) {
            if self.kill_monster(monster, None).is_ok() {
                return true;
            }
        }
        false
    }

    fn find_and_gather(&self) -> bool {
        if let Some(item) = self.conf().target_item {
            if let Some(resource) = self.resources.dropping(&item).first() {
                if self.gather_resource(resource, None).is_ok() {
                    return true;
                }
            }
        }
        if let Some(skill) = self.role().to_skill() {
            if let Some(resource) = self
                .resources
                .highest_providing_exp(self.skill_level(skill), skill)
            {
                if self.gather_resource(resource, None).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if an equipment making the `Character` able to kill the given
    /// `monster` is available, equip it, then move the `Character` to the given
    /// map or the closest containing the `monster` and fight it.
    fn kill_monster(
        &self,
        monster: &MonsterSchema,
        map: Option<&MapSchema>,
    ) -> Result<FightSchema, FightError> {
        let equipment = self.best_available_equipment_against(monster);
        if !self.can_kill_with(monster, &equipment) {
            return Err(FightError::NoEquipmentToKill);
        }
        self.equip_equipment(&equipment);
        if let Some(map) = map {
            self.action_move(map.x, map.y);
        } else if let Some(map) = self.closest_map_with_content_code(&monster.code) {
            self.action_move(map.x, map.y);
        }
        match self.action_fight() {
            Ok(f) => Ok(f),
            Err(e) => Err(FightError::ApiError(e.api_error().unwrap())),
        }
    }

    /// Checks if the character is able to gather the given `resource`. if it
    /// can, equips the best available appropriate tool, then move the `Character`
    /// to the given map or the closest containing the `resource` and gather it.  
    fn gather_resource(
        &self,
        resource: &ResourceSchema,
        map: Option<&MapSchema>,
    ) -> Result<SkillDataSchema, SkillError> {
        if !self.can_gather(resource) {
            return Err(SkillError::InsuffisientSkillLevel);
        }
        if let Some(tool) = self.best_available_tool_for_resource(&resource.code) {
            self.equip_item_from_bank_or_inventory(Slot::Weapon, tool)
        }
        if let Some(map) = map {
            self.action_move(map.x, map.y);
        } else if let Some(map) = self.closest_map_with_content_code(&resource.code) {
            self.action_move(map.x, map.y);
        }
        match self.action_gather() {
            Ok(f) => Ok(f),
            Err(e) => Err(SkillError::ApiError(e.api_error().unwrap())),
        }
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

    // Checks that the `Character` has the required skill level to gather the given `resource`
    fn can_gather(&self, resource: &ResourceSchema) -> bool {
        self.skill_level(resource.skill.into()) >= resource.level
    }

    // Checks that the `Character` has the required skill level to craft the given item `code`
    fn can_craft(&self, code: &str) -> bool {
        if let Some(item) = self.items.get(code) {
            if let Some(skill) = item.skill_to_craft() {
                return self.skill_level(skill) >= item.level;
            }
        }
        false
    }

    /// Returns the current `Equipment` of the `Character`, containing item schemas.
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

    /// Returns the item equiped in the `given` slot.
    fn equiped_in(&self, slot: Slot) -> Option<&ItemSchema> {
        self.data
            .read()
            .map(|d| {
                self.items.get(match slot {
                    Slot::Weapon => &d.weapon_slot,
                    Slot::Shield => &d.shield_slot,
                    Slot::Helmet => &d.helmet_slot,
                    Slot::BodyArmor => &d.body_armor_slot,
                    Slot::LegArmor => &d.leg_armor_slot,
                    Slot::Boots => &d.boots_slot,
                    Slot::Ring1 => &d.ring1_slot,
                    Slot::Ring2 => &d.ring2_slot,
                    Slot::Amulet => &d.amulet_slot,
                    Slot::Artifact1 => &d.artifact1_slot,
                    Slot::Artifact2 => &d.artifact2_slot,
                    Slot::Artifact3 => &d.artifact3_slot,
                    Slot::Consumable1 => &d.consumable1_slot,
                    Slot::Consumable2 => &d.consumable2_slot,
                })
            })
            .ok()?
    }

    /// Withdraw the materials for, craft, then deposit the item `code` until
    /// the given quantity is crafted.
    pub fn craft_items(&self, code: &str, quantity: i32) -> i32 {
        if !self.can_craft(code) {
            error!(
                "{}: doesn't have the required skill level to craft '{}'.",
                self.name, code
            );
            return 0;
        }
        let mut crafted = 0;
        let mut craftable = self.bank.has_mats_for(code);
        info!("{}: is going to craft '{}'x{}", self.name, code, quantity);
        while crafted < quantity && craftable > 0 {
            self.deposit_all();
            crafted += self
                .craft_from_bank(
                    code,
                    min(self.max_current_craftable_items(code), quantity - crafted),
                    PostCraftAction::Deposit,
                )
                .unwrap_or(0);
            craftable = self.bank.has_mats_for(code);
            info!("{}: crafted {}/{} '{}", self.name, crafted, quantity, code)
        }
        if crafted == 0 && self.bank.has_mats_for(code) < quantity {
            return 0;
        }
        quantity
    }

    /// Crafts the maximum amount of given item `code` that can be crafted in one go with the
    /// materials available in bank, then deposit the crafted items.
    pub fn craft_max_from_bank(
        &self,
        code: &str,
        post_action: PostCraftAction,
    ) -> Result<i32, SkillError> {
        let max = self.max_craftable_items_from_bank(code);
        self.craft_from_bank(code, max, post_action)
    }

    /// Crafts the given `quantity` of the given item `code` if the required
    /// materials to craft them in one go are available in bank and deposit the crafted
    /// items into the bank.
    pub fn craft_from_bank(
        &self,
        code: &str,
        quantity: i32,
        post_action: PostCraftAction,
    ) -> Result<i32, SkillError> {
        if !self.can_craft(code) {
            return Err(SkillError::InsuffisientSkillLevel);
        }
        if quantity <= 0 {
            return Err(SkillError::InvalidQuantity);
        }
        if self.max_craftable_items_from_bank(code) < quantity {
            return Err(SkillError::InsuffisientMaterials);
        }
        info!(
            "{}: going to craft '{}'x{} from bank.",
            self.name, code, quantity
        );
        self.deposit_all();
        self.withdraw_mats_for(code, quantity);
        self.action_craft(code, quantity);
        match post_action {
            PostCraftAction::Deposit => self.action_deposit(code, quantity),
            PostCraftAction::Recycle => self.action_recycle(code, quantity),
            PostCraftAction::None => false,
        };
        Ok(quantity)
    }

    /// Deposits all the items to the bank.
    fn deposit_all(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!("{}: depositing all items to the bank.", self.name,);
        for slot in self.inventory_copy() {
            if slot.quantity > 0 {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    /// Deposits all the items of the given `type` to the bank.
    fn deposit_all_of_type(&self, r#type: Type) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!(
            "{}: depositing all items of type '{}' to the bank.",
            self.name, r#type
        );
        for slot in self.inventory_copy() {
            if slot.quantity > 0 && self.items.is_of_type(&slot.code, r#type) {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    /// Deposit all of the given `item` to the bank.
    fn deposit_all_of(&self, code: &str) {
        let amount = self.has_in_inventory(code);
        if amount > 0 {
            let _ = self.action_deposit(code, amount);
        }
    }

    /// Withdraw the materials required to craft the `quantity` of the
    /// item `code` and returns the maximum amount that can be crafted.
    // TODO: add check on `inventory_max_items`
    fn withdraw_mats_for(&self, code: &str, quantity: i32) -> bool {
        let mats = self.items.mats(code);
        for mat in &mats {
            if self.bank.has_item(&mat.code) < mat.quantity * quantity {
                warn!("{}: not enough materials in bank to withdraw the materials required to craft '{code}'x{quantity}", self.name);
                return false;
            }
        }
        info!(
            "{}: going to withdraw materials for '{code}'x{quantity}.",
            self.name
        );
        for mat in &mats {
            let _ = self.action_withdraw(&mat.code, mat.quantity * quantity);
        }
        true
    }

    /// Withdraw the maximum amount of materials considering free spaces in inventory to craft the
    /// maximum amount of the item `code` and returns the maximum amount that can be crafted.
    fn withdraw_max_mats_for(&self, code: &str) -> i32 {
        let max = self.max_current_craftable_items(code);
        self.withdraw_mats_for(code, max);
        max
    }

    /// Calculates the maximum number of items that can be crafted in one go based on
    /// inventory max items
    fn max_craftable_items(&self, code: &str) -> i32 {
        self.inventory_max_items() / self.items.mats_quantity_for(code)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory max items and bank materials.
    fn max_craftable_items_from_bank(&self, code: &str) -> i32 {
        min(
            self.bank.has_mats_for(code),
            self.inventory_max_items() / self.items.mats_quantity_for(code),
        )
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory free space and bank materials.
    fn max_current_craftable_items(&self, code: &str) -> i32 {
        min(
            self.bank.has_mats_for(code),
            self.inventory_free_space() / self.items.mats_quantity_for(code),
        )
    }

    /// Craft the maximum amount of the item `code` with the materials currently available
    /// in the character inventory and returns the amount crafted.
    fn craft_max_from_inventory(&self, code: &str) -> i32 {
        let n = self.has_mats_for(code);
        if n > 0 {
            info!(
                "{}: going to craft all '{}' with materials available in inventory.",
                self.name, code
            );
            self.action_craft(code, n);
            n
        } else {
            error!(
                "{}: not enough materials in inventory to craft {}",
                self.name, code
            );
            0
        }
    }

    /// Reycle the maximum amount of the item `code` with the items  currently
    /// available in the character inventory and returns the amount recycled.
    fn recycle_all(&self, code: &str) -> i32 {
        let n = self.has_in_inventory(code);
        if n > 0 {
            info!("{}: recycling all '{}'.", self.name, code);
            self.action_recycle(code, n);
        }
        n
    }

    fn wait_for_cooldown(&self) {
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        info!(
            "{}: cooling down for {}.{} secondes.",
            self.name,
            s.as_secs(),
            s.subsec_millis()
        );
        sleep(s);
    }

    /// Returns the remaining cooldown duration of the `Character`.
    fn remaining_cooldown(&self) -> Duration {
        if let Some(exp) = self.cooldown_expiration() {
            let synced = Utc::now() - *self.game.server_offset.read().unwrap();
            if synced.cmp(&exp.to_utc()) == Ordering::Less {
                return (exp.to_utc() - synced).to_std().unwrap();
            }
        }
        Duration::from_secs(0)
    }

    /// Returns the cooldown expiration timestamp of the `Character`.
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

    /// Checks if the `Character` inventory is full (all slots are occupied or
    /// `inventory_max_items` is reached).
    fn inventory_is_full(&self) -> bool {
        self.inventory_total() >= self.inventory_max_items()
            || self.data.read().map_or(false, |d| {
                d.inventory.iter().flatten().all(|s| s.quantity > 0)
            })
    }

    /// Returns the amount of the given item `code` in the `Character` inventory.
    pub fn has_in_inventory(&self, code: &str) -> i32 {
        self.data.read().map_or(0, |d| {
            d.inventory
                .iter()
                .flatten()
                .find(|i| i.code == code)
                .map_or(0, |i| i.quantity)
        })
    }

    /// Returns the amount of item in the `Character` inventory.
    fn inventory_total(&self) -> i32 {
        self.data.read().map_or(0, |d| {
            d.inventory.iter().flatten().map(|i| i.quantity).sum()
        })
    }

    /// Returns the maximum number of item the inventory can contain.
    fn inventory_max_items(&self) -> i32 {
        self.data.read().map_or(0, |d| d.inventory_max_items)
    }

    /// Returns the free spaces in the `Character` inventory.
    fn inventory_free_space(&self) -> i32 {
        self.inventory_max_items() - self.inventory_total()
    }

    /// Returns the amount of the given item `code` that can be crafted with
    /// the materials currently in the `Character` inventory.
    fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats(code)
            .iter()
            .map(|mat| self.has_in_inventory(&mat.code) / mat.quantity)
            .min()
            .unwrap_or(0)
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

    /// Return the `ItemSchema` of the raw materials present in the `Character`
    /// inventory.
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

    fn move_to_closest_map_of_type(&self, r#type: &str) {
        if let Some(map) = self.closest_map_of_type(r#type) {
            let (x, y) = (map.x, map.y);
            self.action_move(x, y);
        };
    }

    fn move_to_closest_map_with_content_code(&self, code: &str) {
        if let Some(map) = self.closest_map_with_content_code(code) {
            let (x, y) = (map.x, map.y);
            self.action_move(x, y);
        };
    }

    fn move_to_closest_map_with_content_schema(&self, schema: &MapContentSchema) {
        if let Some(map) = self.closest_map_with_content_schema(schema) {
            let (x, y) = (map.x, map.y);
            self.action_move(x, y);
        };
    }

    /// Returns the closest map from the `Character` containing the given
    /// content `type`.
    fn closest_map_of_type(&self, r#type: &str) -> Option<&MapSchema> {
        let maps = self.maps.of_type(r#type);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    /// Returns the closest map from the `Character` containing the given
    /// content `code`.
    fn closest_map_with_content_code(&self, code: &str) -> Option<&MapSchema> {
        let maps = self.maps.with_ressource(code);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    /// Returns the closest map from the `Character` containing the given
    /// content schema.
    fn closest_map_with_content_schema(&self, schema: &MapContentSchema) -> Option<&MapSchema> {
        let maps = self.maps.with_content_schema(schema);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    /// Returns the closest map from the `Character` among the `maps` given.
    fn closest_map_among<'a>(&'a self, maps: Vec<&'a MapSchema>) -> Option<&MapSchema> {
        let (x, y) = self.position();
        Maps::closest_from_amoung(x, y, maps)
    }

    /// Returns the `Character` position (coordinates).
    fn position(&self) -> (i32, i32) {
        let (x, y) = self.data.read().map_or((0, 0), |d| (d.x, d.y));
        (x, y)
    }

    /// Moves the `Character` to the crafting station corresponding to the skill
    /// required to craft the given item `code`.
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
            if let Some(item) = equipment.slot(s) {
                self.equip_item_from_bank_or_inventory(s, item);
            }
        })
    }

    fn equip_item_from_bank_or_inventory(&self, s: Slot, item: &ItemSchema) {
        let prev_equiped = self.equiped_in(s);
        if prev_equiped.is_some_and(|e| e.code == item.code) {
            return;
        }
        if self.has_in_inventory(&item.code) > 0
            || (self.bank.has_item(&item.code) > 0 && self.action_withdraw(&item.code, 1))
        {
            let _ = self.action_equip(&item.code, s, 1);
            if let Some(i) = prev_equiped {
                let _ = self.action_deposit(&i.code, 1);
            }
        } else {
            error!(
                "{}: item not found in bank or inventory: '{}'.",
                self.name, item.code
            );
        }
    }

    fn best_available_tool_for_resource(&self, code: &str) -> Option<&ItemSchema> {
        match self.resources.get(code) {
            Some(resource) => self
                .items
                .equipable_at_level(self.level(), Slot::Weapon)
                .into_iter()
                .filter(|i| self.has_available(&i.code) > 0)
                .min_by_key(|i| i.skill_cooldown_reduction(Skill::from(resource.skill))),
            None => None,
        }
    }

    fn best_available_equipment_against(&self, monster: &MonsterSchema) -> Equipment {
        let best_equipment = self
            .available_equipable_weapons()
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
            | Slot::Amulet
            | Slot::Boots
            | Slot::Shield => self.best_available_armor_against_with_weapon(slot, monster, weapon),
            _ => None,
        }
    }

    /// Returns the amount of the given item `code` available in bank and inventory.
    fn has_in_bank_or_inv(&self, code: &str) -> i32 {
        self.bank.has_item(code) + self.has_in_inventory(code)
    }

    /// Returns the amount of the given item `code` available in bank, inventory and equipment.
    fn has_available(&self, code: &str) -> i32 {
        self.has_in_bank_or_inv(code) + self.has_equiped(code) as i32
    }

    /// Checks if the given item `code` is equiped.
    fn has_equiped(&self, code: &str) -> usize {
        Slot::iter()
            .filter(|s| self.equiped_in(*s).is_some_and(|e| e.code == code))
            .count()
    }

    /// Returns all the weapons available and equipable by the `Character`
    fn available_equipable_weapons(&self) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.level(), Slot::Weapon)
            .into_iter()
            .filter(|i| self.has_available(&i.code) > 0)
            .collect_vec()
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
        let available = self
            .items
            .equipable_at_level(self.level(), slot)
            .into_iter()
            .filter(|i| {
                self.has_available(&i.code) > {
                    if slot.is_ring_2() {
                        1
                    } else {
                        0
                    }
                }
            })
            .collect_vec();
        let mut upgrade = available.iter().max_by_key(|i| {
            OrderedFloat(self.armor_attack_damage_against_with_weapon(i, monster, weapon))
        });
        if upgrade.is_some_and(|i| i.total_damage_increase() <= 0) {
            upgrade = available
                .iter()
                .min_by_key(|i| OrderedFloat(i.damage_from(monster)))
        }
        upgrade.copied()
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

    /// Refresh the `Character` schema from API.
    fn refresh_data(&self) {
        if let Ok(resp) = self.api.get(&self.name) {
            self.update_data(&resp.data)
        }
    }

    /// Update the `Character` schema with the given `schema.
    fn update_data(&self, schema: &CharacterSchema) {
        if let Ok(mut d) = self.data.write() {
            d.clone_from(schema)
        }
    }

    pub fn toggle_idle(&self) {
        if let Ok(mut conf) = self.conf.write() {
            conf.idle ^= true;
            info!("{} toggled idle: {}.", self.name, conf.idle);
            if !conf.idle {
                self.refresh_data()
            }
        }
    }

    fn role(&self) -> Role {
        self.conf.read().map_or(Role::default(), |d| d.role)
    }

    fn is_gatherer(&self) -> bool {
        matches!(self.role(), Role::Miner | Role::Woodcutter | Role::Fisher)
    }

    fn task(&self) -> String {
        self.data
            .read()
            .map_or("".to_string(), |d| d.task.to_owned())
    }

    fn task_type(&self) -> String {
        self.data
            .read()
            .map_or("".to_string(), |d| d.task_type.to_owned())
    }

    fn task_progress(&self) -> i32 {
        self.data.read().map_or(0, |d| d.task_progress)
    }

    fn task_total(&self) -> i32 {
        self.data.read().map_or(0, |d| d.task_total)
    }

    fn task_missing(&self) -> i32 {
        self.task_total() - self.task_progress()
    }

    fn task_finished(&self) -> bool {
        self.task_progress() >= self.task_total()
    }

    /// Returns the level of the `Character`.
    fn level(&self) -> i32 {
        self.data.read().map_or(1, |d| d.level)
    }

    /// Returns the `Character` level in the given `skill`.
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

    /// Returns the base health of the `Character` without its equipment.
    fn base_health(&self) -> i32 {
        115 + 5 * self.level()
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }
}

#[derive(Debug, Default, PartialEq, Copy, Clone, Deserialize)]
pub enum Role {
    #[default]
    Fighter,
    Miner,
    Woodcutter,
    Fisher,
    Weaponcrafter,
}

impl Role {
    pub fn to_skill(&self) -> Option<Skill> {
        match *self {
            Role::Fighter => None,
            Role::Miner => Some(Skill::Mining),
            Role::Woodcutter => Some(Skill::Woodcutting),
            Role::Fisher => Some(Skill::Fishing),
            Role::Weaponcrafter => Some(Skill::Weaponcrafting),
        }
    }
}
