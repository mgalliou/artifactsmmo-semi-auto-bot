use super::{
    account::Account,
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    average_dmg,
    bank::Bank,
    char_config::CharConfig,
    config::Config,
    equipment::{Equipment, Slot},
    equipment_finder::{EquipmentFinder, Filter},
    events::Events,
    fight_simulator::FightSimulator,
    game::Game,
    items::{DamageType, ItemSource, Items, Type},
    maps::Maps,
    monsters::Monsters,
    orderboard::{Order, OrderBoard},
    resources::Resources,
    skill::Skill,
    ActiveEventSchemaExt, FightSchemaExt, ItemSchemaExt, MonsterSchemaExt, SkillSchemaExt,
};
use crate::artifactsmmo_sdk::char_config::Goal;
use actions::{PostCraftAction, RequestError};
use artifactsmmo_openapi::models::{
    fight_schema, CharacterSchema, FightSchema, InventorySlot, ItemSchema, MapContentSchema,
    MapSchema, MonsterSchema, ResourceSchema, SkillDataSchema,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    cmp::{max, min, Ordering},
    io,
    option::Option,
    sync::{Arc, RwLock},
    thread::{self, sleep, JoinHandle},
    time::Duration,
    vec::Vec,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIs;
mod actions;

pub struct Character {
    pub name: String,
    my_api: MyCharacterApi,
    api: CharactersApi,
    pub account: Arc<Account>,
    game: Arc<Game>,
    maps: Arc<Maps>,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    items: Arc<Items>,
    events: Arc<Events>,
    bank: Arc<Bank>,
    orderboard: Arc<OrderBoard>,
    equipment_finder: EquipmentFinder,
    fight_simulator: FightSimulator,
    pub conf: Arc<RwLock<CharConfig>>,
    pub data: Arc<RwLock<CharacterSchema>>,
}

impl Character {
    pub fn new(
        config: &Config,
        account: &Arc<Account>,
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
            account: account.clone(),
            game: game.clone(),
            maps: game.maps.clone(),
            resources: game.resources.clone(),
            monsters: game.monsters.clone(),
            items: game.items.clone(),
            events: game.events.clone(),
            orderboard: game.orderboard.clone(),
            equipment_finder: EquipmentFinder::new(&game.items),
            fight_simulator: FightSimulator::new(&game.items, &game.monsters),
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
            if self.conf().goals.iter().any(|g| match g {
                Goal::LevelSkills => self.find_and_gather(),
                Goal::LevelUp => self.find_and_kill(),
            }) {
                continue;
            }
            info!("{}: no action found, sleeping for 30sec.", self.name);
            sleep(Duration::from_secs(30));
        }
    }

    fn handle_wooden_stick(&self) {
        if self.conf().skills.contains(&Skill::Combat) && self.has_equiped("wooden_stick") > 0 {
            let _ = self.action_unequip(Slot::Weapon, 1);
            let _ = self.action_deposit("wooden_stick", 1);
        };
    }

    /// If inventory is full, process the raw materials if possible and deposit
    /// all the consumables and resources in inventory to the bank.
    fn process_inventory(&self) {
        if self.inventory_is_full() {
            self.deposit_all();
        }
    }

    /// Returns the next skill that should leveled by the Character, based on
    /// its configuration and the items available in bank.
    fn level_skills_up(&self) -> bool {
        let mut craft_skills = self
            .conf()
            .skills
            .into_iter()
            .filter(|s| !s.is_gathering())
            .collect_vec();
        craft_skills.sort_by_key(|s| self.skill_level(*s));
        craft_skills
            .into_iter()
            .filter(|s| self.skill_level(*s) < if s.is_jewelrycrafting() { 35 } else { 40 })
            .any(|skill| self.level_skill_up(skill, 1))
    }

    fn level_skill_up(&self, skill: Skill, priority: i32) -> bool {
        self.items
            .best_for_leveling_hc(self.skill_level(skill), skill)
            .iter()
            .min_by_key(|i| {
                self.bank.missing_mats_quantity(
                    &i.code,
                    self.max_craftable_items(&i.code),
                    Some(&self.name),
                )
            })
            .is_some_and(|i| {
                match self.craft_from_bank(
                    &i.code,
                    self.max_craftable_items(&i.code),
                    PostCraftAction::Recycle,
                ) {
                    Ok(_) => {
                        info!("{} crafted {} to level up.", self.name, i.code);
                        self.deposit_all();
                        true
                    }
                    Err(e) => {
                        if let CharacterError::InsuffisientMaterials = e {
                            self.bank
                                .missing_mats_for(
                                    &i.code,
                                    self.max_craftable_items(&i.code),
                                    Some(&self.name),
                                )
                                .iter()
                                .for_each(|m| {
                                    self.orderboard.add(Order::new(
                                        &self.name,
                                        &m.code,
                                        m.quantity,
                                        priority,
                                        format!("crafting '{}' to level up {}", i.code, skill),
                                    ))
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

    /// Browser orderboard for completable orders: first check if some orders
    /// can be turned in, then check for completable orders (enough materials to craft all items
    /// from an order. Then check for orders that can be progressed. Then check for order for which
    /// the skill level required needs to be leveled.
    fn handle_orderboard(&self) -> bool {
        if self
            .orderboard
            .orders()
            .into_iter()
            .any(|o| self.turn_in_order(o))
        {
            return true;
        }
        let completable = self.orderboard.orders_filtered(|o| self.can_complete(o));
        if completable.into_iter().any(|r| self.handle_order(r)) {
            return true;
        }
        let mut orders = self.orderboard.orders_filtered(|o| self.can_progress(o));
        let mut skill_up_needed = self
            .orderboard
            .orders_filtered(|o| self.need_to_level_up(o));
        orders.sort_by_key(|o| o.priority);
        orders.reverse();
        if orders.into_iter().any(|r| self.handle_order(r)) {
            return true;
        }
        skill_up_needed.sort_by_key(|o| o.priority);
        skill_up_needed.reverse();
        skill_up_needed.into_iter().any(|r| self.handle_order(r))
    }

    fn can_progress(&self, order: &Order) -> bool {
        self.items.sources_of(&order.item).iter().any(|s| match s {
            ItemSource::Resource(r) => self.can_gather(r).is_ok(),
            ItemSource::Monster(m) => self.can_kill(m).is_ok(),
            ItemSource::Craft => {
                if self.can_craft(&order.item).is_ok() {
                    self.bank
                        .missing_mats_for(&order.item, order.quantity, Some(&self.name))
                        .iter()
                        .for_each(|m| {
                            self.orderboard.add(Order::new(
                                &self.name,
                                &m.code,
                                m.quantity,
                                order.priority + 1,
                                format!("crafting '{}' for order: {}", order.item, order),
                            ))
                        });
                    true
                } else {
                    false
                }
            }
            ItemSource::Task => false,
        })
    }

    fn can_complete(&self, order: &Order) -> bool {
        self.items.sources_of(&order.item).iter().any(|s| match s {
            ItemSource::Resource(_) => false,
            ItemSource::Monster(_) => false,
            ItemSource::Craft => {
                self.can_craft(&order.item).is_ok()
                    && self
                        .bank
                        .missing_mats_for(&order.item, order.quantity, Some(&self.name))
                        .is_empty()
            }
            ItemSource::Task => false,
        })
    }

    fn need_to_level_up(&self, order: &Order) -> bool {
        self.items.sources_of(&order.item).iter().any(|s| match s {
            ItemSource::Craft => match self.can_craft(&order.item) {
                Err(CharacterError::InsuffisientSkillLevel(s, _)) => !s.is_gathering(),
                _ => false,
            },
            _ => false,
        })
    }

    fn handle_order(&self, order: Arc<Order>) -> bool {
        if self.progress_order(&order).is_some() {
            self.turn_in_order(order);
            return true;
        }
        false
    }

    fn progress_order(&self, order: &Order) -> Option<i32> {
        let ret = self
            .items
            .sources_of(&order.item)
            .iter()
            .find_map(|s| match s {
                ItemSource::Resource(r) => {
                    order.inc_worked_by(1);
                    let ret = self
                        .gather_resource(r, None)
                        .ok()
                        .map(|gather| gather.amount_of(&order.item));
                    order.dec_worked_by(1);
                    ret
                }
                ItemSource::Monster(m) => {
                    order.inc_worked_by(1);
                    let ret = self
                        .kill_monster(m, None)
                        .ok()
                        .map(|fight| fight.amount_of(&order.item));
                    order.dec_worked_by(1);
                    ret
                }
                ItemSource::Craft => self.progress_crafting_order(order),
                ItemSource::Task => None,
            });
        if let Some(progress) = ret {
            if progress > 0 {
                info!(
                    "{} progressed by {} on order: {}, in inventories: {}, deposited: {}",
                    self.name,
                    progress,
                    order,
                    self.account.in_inventories(&order.item),
                    order.deposited(),
                );
            }
        }
        ret
    }

    /// Deposit items requiered by the given `order` if needed.
    /// Returns true if items has be deposited.
    fn turn_in_order(&self, order: Arc<Order>) -> bool {
        if self.account.in_inventories(&order.item) >= order.missing()
            || self.inventory_is_full() && !order.turned_in()
        {
            let q = self.has_in_inventory(&order.item);
            if q > 0 {
                self.deposit_all();
                order.inc_deposited(q);
                if order.turned_in() {
                    self.orderboard.remove(&order);
                }
                return true;
            }
        }
        false
    }

    fn progress_crafting_order(&self, order: &Order) -> Option<i32> {
        if order.being_crafted() >= order.missing() {
            return None;
        }
        match self.can_craft(&order.item) {
            Ok(()) => {
                let quantity = min(
                    self.max_craftable_items(&order.item),
                    order.missing()
                        - order.being_crafted()
                        - self.account.in_inventories(&order.item),
                );
                if quantity > 0 {
                    order.inc_being_crafted(quantity);
                    let ret = self.craft_from_bank_unchecked(
                        &order.item,
                        quantity,
                        PostCraftAction::None,
                    );
                    order.dec_being_crafted(quantity);
                    ret.ok();
                }
                None
            }
            Err(e) => match e {
                CharacterError::InsuffisientSkillLevel(s, _) => {
                    if !s.is_gathering() && self.level_skill_up(s, order.priority + 1) {
                        Some(0)
                    } else {
                        None
                    }
                }
                _ => None,
            },
        }
    }

    fn handle_events(&self) -> bool {
        if self.handle_resource_event() {
            return true;
        }
        if self.handle_monster_event() {
            return true;
        }
        false
    }

    fn handle_task(&self) -> bool {
        if self.task().is_empty() || self.task_finished() {
            if self.task_finished() {
                let _ = self.action_complete_task();
            }
            if self.conf().skills.contains(&Skill::Combat) {
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
        let in_bank = self.bank.has_item(&self.task(), Some(&self.name));
        let item = &self.task();
        let missing = self.task_missing();
        let craftable = self.bank.has_mats_for(item, Some(&self.name));

        if in_bank >= missing {
            self.deposit_all();
            let _ = self.action_withdraw(item, max(missing, self.inventory_free_space()));
            return self
                .action_task_trade(item, self.has_in_inventory(&self.task()))
                .is_ok();
        } else if self.can_craft(item).is_ok() && craftable > 0 {
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
        if let Some(monster_code) = &self.conf().target_monster {
            if let Some(monster) = self.monsters.get(monster_code) {
                if self.kill_monster(monster, None).is_ok() {
                    return true;
                }
            }
        }
        if let Some(monster) = self
            .monsters
            .data
            .iter()
            .filter(|m| m.level <= self.level())
            .max_by_key(|m| if self.can_kill(m).is_ok() { m.level } else { 0 })
        {
            info!(
                "{}: found highest killable monster: {}",
                self.name, monster.code
            );
            let _ = self.kill_monster(monster, None);
            return true;
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
        if let Some(skill) = self.conf().skills.iter().find(|s| s.is_gathering()) {
            if let Some(resource) = self
                .resources
                .highest_providing_exp(self.skill_level(*skill), *skill)
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
    ) -> Result<FightSchema, CharacterError> {
        let mut available: Equipment = self.equipment();
        if let Ok(_browsed) = self.bank.browsed.write() {
            match self.can_kill(monster) {
                Ok(equipment) => {
                    available = equipment;
                    self.reserv_equipment(available)
                }
                Err(e) => return Err(e),
            }
            self.order_best_equipment_against(monster, Filter::All);
            self.order_best_equipment_against(monster, Filter::Craftable);
        }
        self.equip_equipment(&available);
        if let Some(map) = map {
            self.action_move(map.x, map.y)?;
        } else if let Some(map) = self.closest_map_with_content_code(&monster.code) {
            self.action_move(map.x, map.y)?;
        }
        Ok(self.action_fight()?)
    }

    /// Checks if the character is able to gather the given `resource`. if it
    /// can, equips the best available appropriate tool, then move the `Character`
    /// to the given map or the closest containing the `resource` and gather it.  
    fn gather_resource(
        &self,
        resource: &ResourceSchema,
        map: Option<&MapSchema>,
    ) -> Result<SkillDataSchema, CharacterError> {
        let mut tool = None;
        self.can_gather(resource)?;
        if let Ok(_browsed) = self.bank.browsed.write() {
            tool = self.best_available_tool_for_resource(&resource.code);
            if let Some(tool) = tool {
                self.reserv_if_needed_and_available(Slot::Weapon, tool);
            }
        }
        if let Some(tool) = tool {
            self.equip_item_from_bank_or_inventory(Slot::Weapon, tool);
        }
        if let Some(map) = map {
            self.action_move(map.x, map.y)?;
        } else if let Some(map) = self.closest_map_with_content_code(&resource.code) {
            self.action_move(map.x, map.y)?;
        }
        Ok(self.action_gather()?)
    }

    /// Checks if the `Character` is able to kill the given monster and returns
    /// the best available equipment to do so.
    fn can_kill<'a>(&'a self, monster: &'a MonsterSchema) -> Result<Equipment<'_>, CharacterError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(CharacterError::SkillDisabled);
        }
        let available = self
            .equipment_finder
            .best_against(self, monster, Filter::Available);
        if self.can_kill_with(monster, &available) {
            Ok(available)
        } else {
            Err(CharacterError::EquipmentTooWeak)
        }
    }

    /// Checks if the `Character` could kill the given `monster` with the given
    /// `equipment`
    fn can_kill_with(&self, monster: &MonsterSchema, equipment: &Equipment) -> bool {
        self.fight_simulator
            .simulate(self.level(), equipment, monster)
            .result
            == fight_schema::Result::Win
    }

    // Checks that the `Character` has the required skill level to gather the given `resource`
    fn can_gather(&self, resource: &ResourceSchema) -> Result<(), CharacterError> {
        let skill: Skill = resource.skill.into();
        if !self.skill_enabled(skill) {
            return Err(CharacterError::SkillDisabled);
        }
        if self.skill_level(skill) < resource.level {
            return Err(CharacterError::InsuffisientSkillLevel(
                skill,
                resource.level,
            ));
        }
        Ok(())
    }

    // Checks that the `Character` has the required skill level to craft the given item `code`
    pub fn can_craft(&self, code: &str) -> Result<(), CharacterError> {
        if let Some(item) = self.items.get(code) {
            if let Some(skill) = item.skill_to_craft() {
                if !self.skill_enabled(skill) {
                    return Err(CharacterError::SkillDisabled);
                }
                if self.skill_level(skill) < item.level {
                    return Err(CharacterError::InsuffisientSkillLevel(skill, item.level));
                }
                return Ok(());
            }
            return Err(CharacterError::ItemNotCraftable);
        }
        Err(CharacterError::ItemNotFound)
    }

    /// Returns the current `Equipment` of the `Character`, containing item schemas.
    pub fn equipment(&self) -> Equipment {
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
        if let Err(e) = self.can_craft(code) {
            error!("{}: {:?}", self.name, e);
            return 0;
        }
        let mut crafted = 0;
        let mut craftable = self.bank.has_mats_for(code, Some(&self.name));
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
            craftable = self.bank.has_mats_for(code, Some(&self.name));
            info!("{}: crafted {}/{} '{}", self.name, crafted, quantity, code)
        }
        if crafted == 0 && self.bank.has_mats_for(code, Some(&self.name)) < quantity {
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
    ) -> Result<i32, CharacterError> {
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
    ) -> Result<i32, CharacterError> {
        self.can_craft(code)?;
        self.craft_from_bank_unchecked(code, quantity, post_action)
    }

    pub fn craft_from_bank_unchecked(
        &self,
        code: &str,
        quantity: i32,
        post_action: PostCraftAction,
    ) -> Result<i32, CharacterError> {
        if self.max_craftable_items_from_bank(code) < quantity {
            return Err(CharacterError::InsuffisientMaterials);
        }
        info!(
            "{}: going to craft '{}'x{} from bank.",
            self.name, code, quantity
        );
        self.deposit_all();
        self.withdraw_mats_for(code, quantity);
        self.action_craft(code, quantity)?;
        //TODO: return errors
        match post_action {
            PostCraftAction::Deposit => {
                let _ = self.action_deposit(code, quantity);
            }
            PostCraftAction::Recycle => {
                let _ = self.action_recycle(code, quantity);
            }
            PostCraftAction::None => (),
        };
        Ok(quantity)
    }

    /// Deposits all the items to the bank.
    /// TODO: add returns type with Result breakdown
    pub fn deposit_all(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!("{}: depositing all items to the bank.", self.name,);
        for slot in self.inventory_copy() {
            if slot.quantity > 0 {
                if let Err(e) = self.action_deposit(&slot.code, slot.quantity) {
                    error!("{}: {:?}", self.name, e)
                }
            }
        }
    }

    pub fn empty_bank(&self) {
        let _ = self.move_to_closest_map_of_type("bank");
        self.deposit_all();
        let content = self.bank.content.read().unwrap().clone();
        content.iter().for_each(|i| {
            info!("{} deleting {:?}", self.name, i);
            let mut remain = i.quantity;
            while remain > 0 {
                let quantity = min(self.inventory_free_space(), remain);
                let _ = self.action_withdraw(&i.code, quantity);
                let _ = self.action_delete(&i.code, quantity);
                remain -= quantity;
            }
        });
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
                if let Err(e) = self.action_deposit(&slot.code, slot.quantity) {
                    error!("{}: {:?}", self.name, e)
                }
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
            if self.bank.has_item(&mat.code, Some(&self.name)) < mat.quantity * quantity {
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
            self.bank.has_mats_for(code, Some(&self.name)),
            self.inventory_max_items() / self.items.mats_quantity_for(code),
        )
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory free space and bank materials.
    fn max_current_craftable_items(&self, code: &str) -> i32 {
        min(
            self.bank.has_mats_for(code, Some(&self.name)),
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
            let _ = self.action_craft(code, n);
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
            let _ = self.action_recycle(code, n);
        }
        n
    }

    fn wait_for_cooldown(&self) {
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        debug!(
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

    fn move_to_closest_map_of_type(&self, r#type: &str) -> Result<MapSchema, CharacterError> {
        if let Some(map) = self.closest_map_of_type(r#type) {
            let (x, y) = (map.x, map.y);
            Ok(self.action_move(x, y)?)
        } else {
            Err(CharacterError::FailedToMove)
        }
    }

    fn move_to_closest_map_with_content_code(
        &self,
        code: &str,
    ) -> Result<MapSchema, CharacterError> {
        if let Some(map) = self.closest_map_with_content_code(code) {
            let (x, y) = (map.x, map.y);
            Ok(self.action_move(x, y)?)
        } else {
            Err(CharacterError::FailedToMove)
        }
    }

    fn move_to_closest_map_with_content_schema(
        &self,
        schema: &MapContentSchema,
    ) -> Result<MapSchema, CharacterError> {
        if let Some(map) = self.closest_map_with_content_schema(schema) {
            let (x, y) = (map.x, map.y);
            Ok(self.action_move(x, y)?)
        } else {
            Err(CharacterError::FailedToMove)
        }
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

    fn map(&self) -> &MapSchema {
        let (x, y) = self.position();
        self.maps.get(x, y).unwrap()
    }

    /// Moves the `Character` to the crafting station corresponding to the skill
    /// required to craft the given item `code`.
    fn move_to_craft(&self, code: &str) -> bool {
        if let Some(dest) = self
            .items
            .skill_to_craft(code)
            .and_then(|s| self.maps.to_craft(s))
        {
            let _ = self.action_move(dest.x, dest.y);
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
            || (self.bank.has_item(&item.code, Some(&self.name)) > 0
                && self.action_withdraw(&item.code, 1).is_ok())
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
            //TODO improve filtering
            Some(resource) => self
                .items
                .equipable_at_level(self.level(), Type::Weapon)
                .into_iter()
                .filter(|i| {
                    i.skill_cooldown_reduction(resource.skill.into()) < 0
                        && self.has_available(&i.code) > 0
                })
                .min_by_key(|i| i.skill_cooldown_reduction(Skill::from(resource.skill))),
            None => None,
        }
    }

    /// Returns the amount of the given item `code` available in bank and inventory.
    fn has_in_bank_or_inv(&self, code: &str) -> i32 {
        self.bank.has_item(code, Some(&self.name)) + self.has_in_inventory(code)
    }

    /// Returns the amount of the given item `code` available in bank, inventory and equipment.
    pub fn has_available(&self, code: &str) -> i32 {
        self.has_in_bank_or_inv(code) + self.has_equiped(code) as i32
    }

    /// Checks if the given item `code` is equiped.
    fn has_equiped(&self, code: &str) -> usize {
        Slot::iter()
            .filter(|s| self.equiped_in(*s).is_some_and(|e| e.code == code))
            .count()
    }

    /// Returns all the weapons available and equipable by the `Character`
    pub fn available_equipable_weapons(&self) -> Vec<&ItemSchema> {
        self.items
            .equipable_at_level(self.level(), Type::Weapon)
            .into_iter()
            .filter(|i| self.has_available(&i.code) > 0)
            .collect_vec()
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
            .map(|t| average_dmg(monster.attack_damage(t), 0, self.resistance(t)))
            .sum()
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

    fn is_gatherer(&self) -> bool {
        self.conf().skills.iter().any(|s| s.is_gathering())
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
    pub fn level(&self) -> i32 {
        self.data.read().map_or(1, |d| d.level)
    }

    /// Returns the `Character` level in the given `skill`.
    fn skill_level(&self, skill: Skill) -> i32 {
        self.data.read().map_or(1, |d| match skill {
            Skill::Combat => d.level,
            Skill::Mining => d.mining_level,
            Skill::Woodcutting => d.woodcutting_level,
            Skill::Fishing => d.fishing_level,
            Skill::Weaponcrafting => d.weaponcrafting_level,
            Skill::Gearcrafting => d.gearcrafting_level,
            Skill::Jewelrycrafting => d.jewelrycrafting_level,
            Skill::Cooking => d.cooking_level,
        })
    }

    /// Returns the base health of the `Character` without its equipment.
    fn base_health(&self) -> i32 {
        115 + 5 * self.level()
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }

    fn order_best_equipment_against(&self, monster: &MonsterSchema, filter: Filter) {
        let equipment = self.equipment_finder.best_against(self, monster, filter);
        if self.can_kill_with(monster, &equipment) {
            self.order_equipment(
                equipment,
                1,
                format!("best {:?} equipment to kill {}", filter, monster.code),
            );
        };
    }

    fn order_equipment(&self, equipment: Equipment<'_>, priority: i32, reason: String) {
        //TODO handle rings correctly
        //TODO handle consumables
        Slot::iter().for_each(|s| {
            if let Some(item) = equipment.slot(s) {
                self.order_if_needed(s, item, priority, reason.clone());
            }
        });
    }

    fn order_if_needed(&self, s: Slot, item: &ItemSchema, priority: i32, reason: String) -> bool {
        if (self.equiped_in(s).is_none()
            || self
                .equiped_in(s)
                .is_some_and(|equiped| item.code != equiped.code))
            && self.has_in_inventory(&item.code) < 1
            && self.bank.has_item(&item.code, Some(&self.name)) < 1
        {
            self.orderboard
                .add(Order::new(&self.name, &item.code, 1, priority, reason));
            return true;
        }
        false
    }

    fn reserv_equipment(&self, equipment: Equipment<'_>) {
        //TODO handle rings correctly
        //TODO handle consumables
        Slot::iter().for_each(|s| {
            if let Some(item) = equipment.slot(s) {
                self.reserv_if_needed_and_available(s, item);
            }
        })
    }

    fn reserv_if_needed_and_available(&self, s: Slot, item: &ItemSchema) {
        if (self.equiped_in(s).is_none()
            || self
                .equiped_in(s)
                .is_some_and(|equiped| item.code != equiped.code))
            && self.has_in_inventory(&item.code) < 1
            && self.bank.has_item(&item.code, Some(&self.name)) > 0
        {
            self.bank.reserv(&item.code, 1, &self.name)
        }
    }

    pub fn unequip_and_deposit_all(&self) {
        let _ = self.move_to_closest_map_of_type("bank");
        //TODO check available space in bank
        Slot::iter().for_each(|s| {
            if let Some(item) = self.equiped_in(s) {
                let quantity = match s {
                    Slot::Consumable1 => self.data.read().unwrap().consumable1_slot_quantity,
                    Slot::Consumable2 => self.data.read().unwrap().consumable2_slot_quantity,
                    _ => 1,
                };
                let _ = self.action_unequip(s, quantity);
                let _ = self.action_deposit(&item.code, quantity);
            }
        })
    }

    fn skill_enabled(&self, s: Skill) -> bool {
        self.conf().skills.contains(&s)
    }
}

#[derive(Debug, Default, PartialEq, Copy, Clone, Deserialize, EnumIs)]
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

#[derive(Debug)]
pub enum CharacterError {
    InsuffisientSkillLevel(Skill, i32),
    InsuffisientMaterials,
    InvalidQuantity,
    NoEquipmentToKill,
    MapNotFound,
    FailedToMove,
    NothingToDeposit,
    RequestError(RequestError),
    SkillDisabled,
    EquipmentTooWeak,
    TooLowLevel,
    ItemNotCraftable,
    ItemNotFound,
}

impl From<RequestError> for CharacterError {
    fn from(value: RequestError) -> Self {
        CharacterError::RequestError(value)
    }
}
