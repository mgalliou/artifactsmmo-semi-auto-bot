use super::{
    account::Account,
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    average_dmg,
    bank::Bank,
    char_config::CharConfig,
    config::Config,
    events::Events,
    fight_simulator::FightSimulator,
    game::Game,
    gear::{Gear, Slot},
    gear_finder::{Filter, GearFinder},
    items::{DamageType, ItemSource, Items, Type},
    maps::Maps,
    monsters::Monsters,
    orderboard::{Order, OrderBoard, Purpose},
    resources::Resources,
    skill::Skill,
    ActiveEventSchemaExt, FightSchemaExt, ItemSchemaExt, MonsterSchemaExt, SkillSchemaExt,
};
use crate::artifactsmmo_sdk::char_config::Goal;
use actions::{PostCraftAction, RequestError};
use artifactsmmo_openapi::models::{
    fight_schema, CharacterSchema, FightSchema, InventorySlot, ItemSchema, MapContentSchema,
    MapSchema, MonsterSchema, ResourceSchema, SimpleItemSchema, SkillDataSchema, TasksRewardSchema,
};
use itertools::Itertools;
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    cmp::min,
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
    gear_finder: GearFinder,
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
            name: data.read().unwrap().name.to_owned(),
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
            gear_finder: GearFinder::new(&game.items),
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
        if let Err(e) = self.handle_wooden_stick() {
            error!("{} error while handling wooden_stick: {:?}", self.name, e)
        };
        loop {
            if self.conf.read().unwrap().idle {
                continue;
            }
            if self.inventory_is_full() {
                self.deposit_all();
                continue;
            }
            self.events.refresh();
            if self.conf().goals.iter().any(|g| match g {
                Goal::Events => self.handle_events(),
                Goal::Orders => self.handle_orderboard(),
                Goal::ReachLevel { level } => self.level() < *level && self.level_combat().is_ok(),
                Goal::ReachSkillLevel { skill, level } => {
                    if self.skill_level(*skill) < *level {
                        self.level_skill_up(*skill);
                        true
                    } else {
                        false
                    }
                }
            }) {
                continue;
            }
            info!("{}: no action found, sleeping for 30sec.", self.name);
            sleep(Duration::from_secs(30));
        }
    }

    fn handle_wooden_stick(&self) -> Result<(), CharacterError> {
        if self.conf().skills.contains(&Skill::Combat) && self.has_equiped("wooden_stick") > 0 {
            self.action_unequip(Slot::Weapon, 1)?;
            self.deposit_item("wooden_stick", 1, None)?;
        };
        Ok(())
    }

    fn level_skill_up(&self, skill: Skill) -> bool {
        self.level_skill_by_crafting(skill) || self.level_skill_by_gathering(&skill).is_ok()
    }

    fn level_skill_by_gathering(&self, skill: &Skill) -> Result<(), CharacterError> {
        let Some(resource) = self
            .resources
            .highest_providing_exp(self.skill_level(*skill), *skill)
        else {
            return Err(CharacterError::RessourceNotFound);
        };
        self.gather_resource(resource, None)?;
        Ok(())
    }

    fn level_skill_by_crafting(&self, skill: Skill) -> bool {
        let Some(item) = self
            .items
            .best_for_leveling(self.skill_level(skill), skill)
            .into_iter()
            .min_by_key(|i| {
                self.bank.missing_mats_quantity(
                    &i.code,
                    self.max_craftable_items(&i.code),
                    Some(&self.name),
                )
            })
        else {
            return false;
        };
        match self.craft_from_bank(
            &item.code,
            self.max_craftable_items(&item.code),
            if skill.is_gathering() {
                PostCraftAction::Deposit
            } else {
                PostCraftAction::Recycle
            },
        ) {
            Ok(n) => {
                info!("{} crafted '{}'x{} to level up.", self.name, item.code, n);
                true
            }
            Err(e) => {
                if let CharacterError::InsuffisientMaterials = e {
                    return !skill.is_gathering()
                        && self.order_missing_mats(
                            &item.code,
                            self.max_craftable_items(&item.code),
                            1,
                            Purpose::Leveling {
                                char: self.name.to_owned(),
                                skill,
                            },
                        );
                }
                false
            }
        }
    }

    /// Browse orderboard for completable orders: first check if some orders
    /// can be turned in, then check for completable orders (enough materials to craft all items
    /// from an order. Then check for orders that can be progressed. Then check for order for which
    /// the skill level required needs to be leveled.
    fn handle_orderboard(&self) -> bool {
        let orders = self.orderboard.orders_by_priority();
        if orders.iter().cloned().any(|o| self.turn_in_order(o)) {
            return true;
        }
        let mut completable = orders.iter().filter(|o| self.can_complete(o)).cloned();
        if completable.any(|r| self.handle_order(r)) {
            return true;
        }
        let mut progressable = orders.into_iter().filter(|o| self.can_progress(o));
        if progressable.any(|r| self.handle_order(r)) {
            return true;
        }
        false
    }

    fn can_progress(&self, order: &Order) -> bool {
        self.items.sources_of(&order.item).iter().any(|s| match s {
            ItemSource::Resource(r) => self.can_gather(r).is_ok(),
            ItemSource::Monster(m) => self.can_kill(m).is_ok(),
            ItemSource::Craft => self.can_craft(&order.item).is_ok_and(|_| {
                if order.being_crafted() <= 0 {
                    // NOTE: Maybe ordering missing mats should be done elsewhere
                    self.order_missing_mats(
                        &order.item,
                        order.missing() - self.account.in_inventories(&order.item),
                        order.priority,
                        order.purpose.clone(),
                    );
                };
                true
            }),
            ItemSource::TaskReward => order.worked_by() <= 0,
            ItemSource::Task => true,
        })
    }

    /// Creates orders based on the missing (not available in bank) materials requiered to craft
    /// the `quantity` of the given `item`. Orders are created with the given `priority` and
    /// `purpose`. Returns true if an order has been made.
    fn order_missing_mats(
        &self,
        item: &str,
        quantity: i32,
        priority: i32,
        purpose: Purpose,
    ) -> bool {
        let mut ordered: bool = false;
        self.bank
            .missing_mats_for(item, quantity, Some(&self.name))
            .iter()
            .for_each(|m| {
                if self.orderboard.add(Order::new(
                    None,
                    &m.code,
                    m.quantity,
                    priority,
                    purpose.clone(),
                )) {
                    ordered = true
                }
            });
        ordered
    }

    fn can_complete(&self, order: &Order) -> bool {
        self.items.sources_of(&order.item).iter().any(|s| match s {
            ItemSource::Resource(_) => false,
            ItemSource::Monster(_) => false,
            ItemSource::Craft => {
                self.can_craft(&order.item).is_ok()
                    && self
                        .bank
                        .missing_mats_for(&order.item, order.quantity(), Some(&self.name))
                        .is_empty()
            }
            ItemSource::TaskReward => self.bank.has_item("tasks_coin", Some(&self.name)) >= 6,
            ItemSource::Task => {
                self.bank.has_item(&self.task(), Some(&self.name)) >= self.task_missing()
            }
        })
    }

    fn handle_order(&self, order: Arc<Order>) -> bool {
        let Some(progress) = self.progress_order(&order) else {
            return false;
        };
        if progress > 0 {
            info!(
                "{}: progressed by {} on order: {}, in inventories: {}",
                self.name,
                progress,
                order,
                self.account.in_inventories(&order.item),
            );
        }
        self.turn_in_order(order);
        true
    }

    fn progress_order(&self, order: &Order) -> Option<i32> {
        self.items
            .sources_of(&order.item)
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
                ItemSource::Craft => self.progress_crafting_order(order),
                ItemSource::TaskReward => self.progress_task_reward_order(order),
                ItemSource::Task => self.progress_task_order(),
            })
    }

    fn progress_task_reward_order(&self, order: &Order) -> Option<i32> {
        if order.worked_by() > 0 {
            return None;
        }
        order.inc_worked_by(1);
        let ret = match self.exchange_task() {
            Ok(r) => Some(if r.code == order.item { r.quantity } else { 0 }),
            Err(e) => {
                if let CharacterError::NotEnoughCoin = e {
                    let q = 6 - self.bank.has_item("tasks_coin", Some(&self.name));
                    if self.orderboard.add(Order::new(
                        None,
                        "tasks_coin",
                        q,
                        1,
                        order.purpose.to_owned(),
                    )) {
                        return Some(0);
                    }
                }
                None
            }
        };
        order.dec_worked_by(1);
        ret
    }

    fn progress_task_order(&self) -> Option<i32> {
        match self.complete_task() {
            Ok(r) => Some(r),
            Err(e) => {
                if let CharacterError::NoTask = e {
                    if let Err(e) = self.action_accept_task("items") {
                        error!("{} error while accepting new task: {:?}", self.name, e)
                    }
                    Some(0)
                } else if let CharacterError::TaskNotFinished = e {
                    if self.progress_task() {
                        Some(0)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    fn exchange_task(&self) -> Result<TasksRewardSchema, CharacterError> {
        if self.bank.reserv("tasks_coin", 6, &self.name).is_err() {
            return Err(CharacterError::NotEnoughCoin);
        }
        self.deposit_all();
        self.withdraw_item("tasks_coin", 6)?;
        self.action_task_exchange().map_err(|e| e.into())
    }

    /// Deposit items requiered by the given `order` if needed.
    /// Returns true if items has be deposited.
    fn turn_in_order(&self, order: Arc<Order>) -> bool {
        if (!order.turned_in()
            && self.account.in_inventories(&order.item) + order.being_crafted() >= order.missing())
            || self.inventory_is_full()
        {
            return self.deposit_order(&order);
        }
        false
    }

    fn deposit_order(&self, order: &Order) -> bool {
        let q = self.has_in_inventory(&order.item);
        if q > 0
            && self
                .deposit_item(&order.item, min(q, order.missing()), order.owner.clone())
                .is_ok()
        {
            order.inc_deposited(q);
            if order.turned_in() {
                self.orderboard.remove(order);
            }
            return true;
        }
        false
    }

    fn progress_crafting_order(&self, order: &Order) -> Option<i32> {
        match self.can_craft(&order.item) {
            Ok(()) => {
                if order.being_crafted()
                    < order.missing() - self.account.in_inventories(&order.item)
                {
                    let quantity = min(
                        self.max_craftable_items(&order.item),
                        order.missing()
                            - order.being_crafted()
                            - self.account.in_inventories(&order.item),
                    );
                    if quantity > 0 {
                        order.inc_being_crafted(quantity);
                        let crafted =
                            self.craft_from_bank(&order.item, quantity, PostCraftAction::None);
                        order.dec_being_crafted(quantity);
                        return crafted.ok();
                    }
                }
            }
            Err(e) => {
                if let CharacterError::InsuffisientMaterials = e {
                    if order.being_crafted() <= 0
                        && self.order_missing_mats(
                            &order.item,
                            order.missing() - self.account.in_inventories(&order.item),
                            order.priority,
                            order.purpose.clone(),
                        )
                    {
                        return Some(0);
                    }
                }
            }
        }
        None
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

    fn progress_task(&self) -> bool {
        if let Some(monster) = self.monsters.get(&self.task()) {
            if self.kill_monster(monster, None).is_ok() {
                return true;
            }
        }
        if self.progress_item_task() {
            return true;
        }
        false
    }

    fn progress_item_task(&self) -> bool {
        let item = &self.task();
        let in_bank = self.bank.has_item(&self.task(), Some(&self.name));
        let missing = self.task_missing();

        if in_bank >= missing {
            let q = min(missing, self.inventory_max_items());
            if let Err(e) = self.bank.reserv(item, q, &self.name) {
                error!(
                    "{}: error while reserving items for item task: {:?}",
                    self.name, e
                )
            }
            self.deposit_all();
            if let Err(e) = self.withdraw_item(item, q) {
                error!("{}: error while withdrawing {:?}", self.name, e);
                self.bank.decrease_reservation(item, q, &self.name);
            };
            self.action_task_trade(item, q).is_ok()
        } else {
            self.orderboard.add(Order::new(
                Some(&self.name),
                &self.task(),
                missing - self.bank.has_item(item, Some(&self.name)),
                1,
                Purpose::Task {
                    char: self.name.to_owned(),
                },
            ));
            false
        }
    }

    fn complete_task(&self) -> Result<i32, CharacterError> {
        if self.task().is_empty() {
            return Err(CharacterError::NoTask);
        }
        if !self.task_finished() {
            return Err(CharacterError::TaskNotFinished);
        }
        self.action_complete_task()
            .map(|r| r.quantity)
            .map_err(|e| e.into())
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

    /// Find a target and kill it if possible.
    fn level_combat(&self) -> Result<(), CharacterError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(CharacterError::SkillDisabled);
        }
        let Some(monster) = self
            .monsters
            .data
            .iter()
            .filter(|m| m.level <= self.level())
            .max_by_key(|m| if self.can_kill(m).is_ok() { m.level } else { 0 })
        else {
            return Err(CharacterError::MonsterNotFound);
        };
        info!(
            "{}: found highest killable monster: {}",
            self.name, monster.code
        );
        self.kill_monster(monster, None)?;
        Ok(())
    }

    /// Checks if an gear making the `Character` able to kill the given
    /// `monster` is available, equip it, then move the `Character` to the given
    /// map or the closest containing the `monster` and fight it.
    fn kill_monster(
        &self,
        monster: &MonsterSchema,
        map: Option<&MapSchema>,
    ) -> Result<FightSchema, CharacterError> {
        let mut available: Gear = self.gear();
        if let Ok(_browsed) = self.bank.browsed.write() {
            match self.can_kill(monster) {
                Ok(gear) => {
                    available = gear;
                    self.reserv_gear(available)
                }
                Err(e) => return Err(e),
            }
            self.order_best_gear_against(monster, Filter::Craftable);
        }
        self.equip_gear(&available);
        if let Some(map) = map {
            self.action_move(map.x, map.y)?;
        } else if let Some(map) = self.closest_map_with_content_code(&monster.code) {
            self.action_move(map.x, map.y)?;
        }
        Ok(self.action_fight()?)
    }

    /// Checks if the character is able to gather the given `resource`. If it
    /// can, equips the best available appropriate tool, then move the `Character`
    /// to the given map or the closest containing the `resource` and gather it.  
    fn gather_resource(
        &self,
        resource: &ResourceSchema,
        map: Option<&MapSchema>,
    ) -> Result<SkillDataSchema, CharacterError> {
        self.can_gather(resource)?;
        self.check_for_tool(resource);
        if let Some(map) = map {
            self.action_move(map.x, map.y)?;
        } else if let Some(map) = self.closest_map_with_content_code(&resource.code) {
            self.action_move(map.x, map.y)?;
        }
        Ok(self.action_gather()?)
    }

    /// TODO: handle available tool versus best craftable
    fn check_for_tool(&self, resource: &ResourceSchema) {
        let Some(tool) = self.best_tool_for_resource(&resource.code) else {
            return;
        };
        let Ok(_browsed) = self.bank.browsed.write() else {
            return;
        };
        if self.has_available(&tool.code) > 0 {
            self.reserv_if_needed_and_available(Slot::Weapon, tool);
        } else {
            self.orderboard.add(Order::new(
                Some(&self.name),
                &tool.code,
                1,
                1,
                Purpose::Gear {
                    char: self.name.to_owned(),
                    slot: Slot::Weapon,
                    item_code: tool.code.to_owned(),
                },
            ));
        }
        if self.has_available(&tool.code) > 0 {
            self.equip_item_from_bank_or_inventory(Slot::Weapon, tool);
        }
    }

    /// Checks if the `Character` is able to kill the given monster and returns
    /// the best available gear to do so.
    fn can_kill<'a>(&'a self, monster: &'a MonsterSchema) -> Result<Gear<'_>, CharacterError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(CharacterError::SkillDisabled);
        }
        if self.inventory_is_full() {
            return Err(CharacterError::InventoryFull);
        }
        let available = self
            .gear_finder
            .best_against(self, monster, Filter::Available);
        if self.can_kill_with(monster, &available) {
            Ok(available)
        } else {
            Err(CharacterError::GearTooWeak)
        }
    }

    /// Checks if the `Character` could kill the given `monster` with the given
    /// `gear`
    fn can_kill_with(&self, monster: &MonsterSchema, gear: &Gear) -> bool {
        self.fight_simulator
            .simulate(self.level(), gear, monster)
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
        if self.inventory_is_full() {
            return Err(CharacterError::InventoryFull);
        }
        Ok(())
    }

    // Checks that the `Character` has the required skill level to craft the given item `code`
    pub fn can_craft(&self, code: &str) -> Result<(), CharacterError> {
        let Some(item) = self.items.get(code) else {
            return Err(CharacterError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CharacterError::ItemNotCraftable);
        };
        if !self.skill_enabled(skill) {
            return Err(CharacterError::SkillDisabled);
        }
        if self.skill_level(skill) < item.level {
            return Err(CharacterError::InsuffisientSkillLevel(skill, item.level));
        }
        // TODO: improve condition
        if self.inventory_is_full() {
            return Err(CharacterError::InventoryFull);
        }
        Ok(())
    }

    /// Returns the current `Gear` of the `Character`, containing item schemas.
    pub fn gear(&self) -> Gear {
        let d = self.data.read().unwrap();
        Gear {
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
        }
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
        self.withdraw_mats_for(code, quantity)?;
        self.move_to_craft(code)?;
        self.action_craft(code, quantity)?;
        //TODO: return errors
        match post_action {
            PostCraftAction::Deposit => {
                if let Err(e) = self.deposit_item(code, quantity, None) {
                    error!(
                        "{}: error while depositing items after crafting from bank: {:?}",
                        self.name, e
                    )
                }
            }
            PostCraftAction::Recycle => {
                if let Err(e) = self.move_to_craft(code) {
                    error!(
                        "{}: failed to move before recycling items after crafting from bank: {:?}",
                        self.name, e
                    )
                } else if let Err(e) = self.action_recycle(code, quantity) {
                    error!(
                        "{}: error while recycling items after crafting from bank: {:?}",
                        self.name, e
                    )
                }
            }
            PostCraftAction::None => (),
        };
        Ok(quantity)
    }

    pub fn deposit_item(
        &self,
        item: &str,
        quantity: i32,
        owner: Option<String>,
    ) -> Result<SimpleItemSchema, CharacterError> {
        if self.has_in_inventory(item) < quantity {
            // TODO: return a better error
            return Err(CharacterError::ItemNotFound);
        }
        self.move_to_closest_map_of_type("bank")?;
        if self.bank.free_slots() <= 3 {
            if let Err(e) = self.expand_bank() {
                error!("{}: failed to expand bank capacity: {:?}", self.name, e)
            }
        }
        let deposit = self.action_deposit(item, quantity);
        if deposit.is_ok() {
            if let Some(owner) = owner {
                if let Err(e) = self.bank.reserv(item, quantity, &owner) {
                    error!("{}: failed to reserv deposited item: {:?}", self.name, e)
                }
            }
        }
        if let Err(e) = self.deposit_all_gold() {
            error!("{}: failed to deposit gold to the bank: {:?}", self.name, e)
        }
        Ok(deposit?)
    }

    pub fn withdraw_item(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, CharacterError> {
        if self.bank.has_item(item, Some(&self.name)) < quantity {
            // TODO: return a better error
            return Err(CharacterError::ItemNotFound);
        }
        self.move_to_closest_map_of_type("bank")?;
        let deposit = self.action_withdraw(item, quantity);
        if deposit.is_ok() {
            self.bank.decrease_reservation(item, quantity, &self.name);
        }
        Ok(deposit?)
    }

    /// Deposits all the gold and items in the character inventory into the bank.
    /// Items needed by orders are turned in first.
    /// Bank is expanded if close to being full.
    /// TODO: add returns type with Result breakdown
    pub fn deposit_all(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!("{}: going to deposit all items to the bank.", self.name,);
        self.orderboard.orders_by_priority().iter().for_each(|o| {
            self.deposit_order(o);
        });
        self.inventory_copy().iter().for_each(|slot| {
            if slot.quantity > 0 {
                if let Err(e) = self.deposit_item(&slot.code, slot.quantity, None) {
                    error!("{}: {:?}", self.name, e)
                }
            }
        });
    }

    pub fn deposit_all_gold(&self) -> Result<i32, CharacterError> {
        let gold = self.data.read().unwrap().gold;
        if gold <= 0 {
            return Ok(0);
        };
        Ok(self.action_deposit_gold(gold)?)
    }

    pub fn expand_bank(&self) -> Result<i32, CharacterError> {
        let Ok(_being_expanded) = self.bank.being_expanded.try_write() else {
            return Err(CharacterError::BankUnavailable);
        };
        if self.bank.gold() + self.gold() < self.bank.next_expansion_cost() {
            return Err(CharacterError::InsuffisientGold);
        };
        self.move_to_closest_map_of_type("bank")?;
        self.action_withdraw_gold(self.bank.next_expansion_cost() - self.gold())?;
        Ok(self.action_expand_bank()?)
    }

    pub fn empty_bank(&self) {
        if let Err(e) = self.move_to_closest_map_of_type("bank") {
            error!(
                "{} failed to move to bank before emptying bank: {:?}",
                self.name, e
            )
        }
        self.deposit_all();
        let content = self.bank.content.read().unwrap().clone();
        content.iter().for_each(|i| {
            info!("{} deleting {:?}", self.name, i);
            let mut remain = i.quantity;
            while remain > 0 {
                let quantity = min(self.inventory_free_space(), remain);
                if let Err(e) = self.withdraw_item(&i.code, quantity) {
                    error!(
                        "{} error while withdrawing item during bank empting: {:?}",
                        self.name, e
                    )
                }
                if let Err(e) = self.action_delete(&i.code, quantity) {
                    error!(
                        "{} error while delete item during bank emptying: {:?}",
                        self.name, e
                    )
                }
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
                if let Err(e) = self.deposit_item(&slot.code, slot.quantity, None) {
                    error!("{}: {:?}", self.name, e)
                }
            }
        }
    }

    /// Withdraw the materials required to craft the `quantity` of the
    /// item `code` and returns the maximum amount that can be crafted.
    // TODO: add check on `inventory_max_items`
    fn withdraw_mats_for(&self, code: &str, quantity: i32) -> Result<(), CharacterError> {
        let mats = self.items.mats(code);
        for mat in &mats {
            if self.bank.has_item(&mat.code, Some(&self.name)) < mat.quantity * quantity {
                warn!("{}: not enough materials in bank to withdraw the materials required to craft '{code}'x{quantity}", self.name);
                return Err(CharacterError::InsuffisientMaterials);
            }
        }
        info!(
            "{}: going to withdraw materials for '{code}'x{quantity}.",
            self.name
        );
        for mat in &mats {
            self.withdraw_item(&mat.code, mat.quantity * quantity)?;
        }
        Ok(())
    }

    /// Withdraw the maximum amount of materials considering free spaces in inventory to craft the
    /// maximum amount of the item `code` and returns the maximum amount that can be crafted.
    fn withdraw_max_mats_for(&self, code: &str) -> Result<i32, CharacterError> {
        let max = self.max_current_craftable_items(code);
        self.withdraw_mats_for(code, max)?;
        Ok(max)
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
    fn craft_max_from_inventory(&self, code: &str) -> Result<(), CharacterError> {
        let n = self.has_mats_for(code);
        if 0 < n {
            Err(CharacterError::InsuffisientMaterials)
        } else {
            info!(
                "{}: going to craft all '{}' with materials available in inventory.",
                self.name, code
            );
            self.move_to_craft(code)?;
            self.action_craft(code, n)?;
            Ok(())
        }
    }

    /// Reycle the maximum amount of the item `code` with the items  currently
    /// available in the character inventory and returns the amount recycled.
    pub fn recycle_all(&self, code: &str) -> i32 {
        let n = self.has_in_inventory(code);
        if n > 0 {
            info!("{}: recycling all '{}'.", self.name, code);
            if let Err(e) = self.action_recycle(code, n) {
                error!(
                    "{}: error while recycling all '{}': {:?}",
                    self.name, code, e
                )
            }
        }
        n
    }

    fn gold(&self) -> i32 {
        self.data.read().unwrap().gold
    }

    /// Checks if the `Character` inventory is full (all slots are occupied or
    /// `inventory_max_items` is reached).
    fn inventory_is_full(&self) -> bool {
        self.inventory_total() >= self.inventory_max_items()
            || self
                .data
                .read()
                .unwrap()
                .inventory
                .iter()
                .flatten()
                .all(|s| s.quantity > 0)
    }

    /// Returns the amount of the given item `code` in the `Character` inventory.
    pub fn has_in_inventory(&self, code: &str) -> i32 {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .find(|i| i.code == code)
            .map_or(0, |i| i.quantity)
    }

    /// Returns the amount of item in the `Character` inventory.
    fn inventory_total(&self) -> i32 {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .map(|i| i.quantity)
            .sum()
    }

    /// Returns the maximum number of item the inventory can contain.
    fn inventory_max_items(&self) -> i32 {
        self.data.read().unwrap().inventory_max_items
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
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .cloned()
            .collect_vec()
    }

    /// Return the `ItemSchema` of the raw materials present in the `Character`
    /// inventory.
    fn inventory_raw_mats(&self) -> Vec<&ItemSchema> {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .filter_map(|slot| self.items.get(&slot.code))
            .filter(|i| i.is_raw_mat())
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
        let Some(map) = self.closest_map_with_content_code(code) else {
            return Err(CharacterError::FailedToMove);
        };
        let (x, y) = (map.x, map.y);
        Ok(self.action_move(x, y)?)
    }

    fn move_to_closest_map_with_content_schema(
        &self,
        schema: &MapContentSchema,
    ) -> Result<MapSchema, CharacterError> {
        let Some(map) = self.closest_map_with_content_schema(schema) else {
            return Err(CharacterError::FailedToMove);
        };
        let (x, y) = (map.x, map.y);
        Ok(self.action_move(x, y)?)
    }

    /// Returns the closest map from the `Character` containing the given
    /// content `type`.
    fn closest_map_of_type(&self, r#type: &str) -> Option<Arc<MapSchema>> {
        let maps = self.maps.of_type(r#type);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    /// Returns the closest map from the `Character` containing the given
    /// content `code`.
    fn closest_map_with_content_code(&self, code: &str) -> Option<Arc<MapSchema>> {
        let maps = self.maps.with_ressource(code);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    /// Returns the closest map from the `Character` containing the given
    /// content schema.
    fn closest_map_with_content_schema(&self, schema: &MapContentSchema) -> Option<Arc<MapSchema>> {
        let maps = self.maps.with_content_schema(schema);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    /// Returns the closest map from the `Character` among the `maps` given.
    fn closest_map_among(&self, maps: Vec<Arc<MapSchema>>) -> Option<Arc<MapSchema>> {
        let (x, y) = self.position();
        Maps::closest_from_amoung(x, y, maps)
    }

    /// Returns the `Character` position (coordinates).
    fn position(&self) -> (i32, i32) {
        let d = self.data.read().unwrap();
        let (x, y) = (d.x, d.y);
        (x, y)
    }

    fn map(&self) -> Arc<MapSchema> {
        let (x, y) = self.position();
        self.maps.get(x, y).unwrap()
    }

    /// Moves the `Character` to the crafting station corresponding to the skill
    /// required to craft the given item `code`.
    fn move_to_craft(&self, code: &str) -> Result<(), CharacterError> {
        let Some(dest) = self
            .items
            .skill_to_craft(code)
            .and_then(|s| self.maps.to_craft(s))
        else {
            return Err(CharacterError::MapNotFound);
        };
        self.action_move(dest.x, dest.y)?;
        Ok(())
    }

    fn equip_gear(&self, gear: &Gear) {
        Slot::iter().for_each(|s| {
            if let Some(item) = gear.slot(s) {
                self.equip_item_from_bank_or_inventory(s, item);
            }
        })
    }

    fn equip_item_from_bank_or_inventory(&self, s: Slot, item: &ItemSchema) {
        let prev_equiped = self.equiped_in(s);
        if prev_equiped.is_some_and(|e| e.code == item.code) {
            return;
        }
        if self.has_in_inventory(&item.code) <= 0
            && self.bank.has_item(&item.code, Some(&self.name)) > 0
        {
            self.deposit_all();
            if let Err(e) = self.withdraw_item(&item.code, 1) {
                error!(
                    "{} failed withdraw item from bank or inventory: {:?}",
                    self.name, e
                );
            }
        }
        if let Err(e) = self.action_equip(&item.code, s, 1) {
            error!(
                "{} failed to equip item from bank or inventory: {:?}",
                self.name, e
            );
        }
        if let Some(i) = prev_equiped {
            if let Err(e) = self.deposit_item(&i.code, 1, None) {
                error!(
                    "{} failed to deposit previously equiped item: {:?}",
                    self.name, e
                );
            }
        }
    }

    fn best_tool_for_resource(&self, code: &str) -> Option<&ItemSchema> {
        match self.resources.get(code) {
            //TODO improve filtering
            Some(resource) => self
                .items
                .equipable_at_level(self.level(), Type::Weapon)
                .into_iter()
                .filter(|i| i.skill_cooldown_reduction(resource.skill.into()) < 0)
                .min_by_key(|i| i.skill_cooldown_reduction(Skill::from(resource.skill))),
            None => None,
        }
    }

    /// Returns the amount of the given item `code` available in bank and inventory.
    fn has_in_bank_or_inv(&self, code: &str) -> i32 {
        self.bank.has_item(code, Some(&self.name)) + self.has_in_inventory(code)
    }

    /// Returns the amount of the given item `code` available in bank, inventory and gear.
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
        let d = self.data.read().unwrap();
        match r#type {
            DamageType::Air => d.res_air,
            DamageType::Earth => d.res_earth,
            DamageType::Fire => d.res_fire,
            DamageType::Water => d.res_water,
        }
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
        self.data.write().unwrap().clone_from(schema)
    }

    pub fn toggle_idle(&self) {
        let mut conf = self.conf.write().unwrap();
        conf.idle ^= true;
        info!("{} toggled idle: {}.", self.name, conf.idle);
        if !conf.idle {
            self.refresh_data()
        }
    }

    fn is_gatherer(&self) -> bool {
        self.conf().skills.iter().any(|s| s.is_gathering())
    }

    fn task(&self) -> String {
        self.data.read().unwrap().task.to_owned()
    }

    fn task_type(&self) -> String {
        self.data.read().unwrap().task_type.to_owned()
    }

    fn task_progress(&self) -> i32 {
        self.data.read().unwrap().task_progress
    }

    fn task_total(&self) -> i32 {
        self.data.read().unwrap().task_total
    }

    fn task_missing(&self) -> i32 {
        self.task_total() - self.task_progress()
    }

    fn task_finished(&self) -> bool {
        self.task_progress() >= self.task_total()
    }

    /// Returns the level of the `Character`.
    pub fn level(&self) -> i32 {
        self.data.read().unwrap().level
    }

    /// Returns the `Character` level in the given `skill`.
    fn skill_level(&self, skill: Skill) -> i32 {
        let d = self.data.read().unwrap();
        match skill {
            Skill::Combat => d.level,
            Skill::Mining => d.mining_level,
            Skill::Woodcutting => d.woodcutting_level,
            Skill::Fishing => d.fishing_level,
            Skill::Weaponcrafting => d.weaponcrafting_level,
            Skill::Gearcrafting => d.gearcrafting_level,
            Skill::Jewelrycrafting => d.jewelrycrafting_level,
            Skill::Cooking => d.cooking_level,
        }
    }

    /// Returns the base health of the `Character` without its gear.
    fn base_health(&self) -> i32 {
        115 + 5 * self.level()
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }

    fn order_best_gear_against(&self, monster: &MonsterSchema, filter: Filter) {
        let gear = self.gear_finder.best_against(self, monster, filter);
        if self.can_kill_with(monster, &gear) {
            self.order_gear(gear, 1);
        };
    }

    fn order_gear(&self, gear: Gear<'_>, priority: i32) {
        //TODO handle rings correctly
        //TODO handle consumables
        Slot::iter().for_each(|s| {
            if let Some(item) = gear.slot(s) {
                self.order_if_needed(s, item, priority);
            }
        });
    }

    fn order_if_needed(&self, slot: Slot, item: &ItemSchema, priority: i32) -> bool {
        if (self.equiped_in(slot).is_none()
            || self
                .equiped_in(slot)
                .is_some_and(|equiped| item.code != equiped.code))
            && self.has_in_inventory(&item.code) < 1
            && self.bank.has_item(&item.code, Some(&self.name)) < 1
        {
            self.orderboard.add(Order::new(
                Some(&self.name),
                &item.code,
                1,
                priority,
                Purpose::Gear {
                    char: self.name.to_owned(),
                    slot,
                    item_code: item.code.to_owned(),
                },
            ));
            return true;
        }
        false
    }

    fn reserv_gear(&self, gear: Gear<'_>) {
        //TODO handle rings correctly
        //TODO handle consumables
        Slot::iter().for_each(|s| {
            if let Some(item) = gear.slot(s) {
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
        {
            if let Err(e) = self.bank.reserv(&item.code, 1, &self.name) {
                error!("{} failed to reserv '{}': {:?}", self.name, item.code, e)
            }
        }
    }

    pub fn unequip_and_deposit_all(&self) {
        Slot::iter().for_each(|s| {
            if let Some(item) = self.equiped_in(s) {
                let quantity = match s {
                    Slot::Consumable1 => self.data.read().unwrap().consumable1_slot_quantity,
                    Slot::Consumable2 => self.data.read().unwrap().consumable2_slot_quantity,
                    _ => 1,
                };
                if let Err(e) = self.action_unequip(s, quantity) {
                    error!(
                        "{}: failed to unequip '{}'x{} during unequip_and_deposit_all: {:?}",
                        self.name, &item.code, quantity, e
                    )
                } else if let Err(e) = self.deposit_item(&item.code, quantity, None) {
                    error!(
                        "{}: failed to deposit '{}'x{} during `unequip_and_deposit_all`: {:?}",
                        self.name, &item.code, quantity, e
                    )
                }
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
    NoGearToKill,
    MapNotFound,
    FailedToMove,
    NothingToDeposit,
    RequestError(RequestError),
    SkillDisabled,
    GearTooWeak,
    TooLowLevel,
    ItemNotCraftable,
    ItemNotFound,
    NoTask,
    TaskNotFinished,
    NotEnoughCoin,
    InsuffisientGold,
    BankUnavailable,
    InventoryFull,
    RessourceNotFound,
    MonsterNotFound,
    EventNotFound,
    NoOrderFullfilable,
    NotGoalFullfilable,
}

impl From<RequestError> for CharacterError {
    fn from(value: RequestError) -> Self {
        CharacterError::RequestError(value)
    }
}
