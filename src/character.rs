use crate::{
    account::ACCOUNT,
    bank::BANK,
    bot_config::{CharConfig, Goal, BOT_CONFIG},
    gear_finder::{Filter, GEAR_FINDER},
    inventory::Inventory,
    leveling_helper::LEVELING_HELPER,
    orderboard::{Order, Purpose, ORDER_BOARD},
};
use artifactsmmo_sdk::{
    char::{
        request_handler::{CharacterRequestHandler, RequestError},
        CharacterData, HasCharacterData, Skill,
    },
    consts::{
        BANK_MIN_FREE_SLOT, CRAFT_TIME, MAX_LEVEL, MIN_COIN_THRESHOLD, MIN_FOOD_THRESHOLD,
        TASKS_COIN, TASK_CANCEL_PRICE, TASK_EXCHANGE_PRICE,
    },
    fight_simulator::FIGHT_SIMULATOR,
    gear::{Gear, Slot},
    items::{ItemSchemaExt, ItemSource, Type},
    maps::{ContentType, MapSchemaExt},
    models::{
        CharacterSchema, FightResult, FightSchema, ItemSchema, MapSchema,
        MonsterSchema, RecyclingItemsSchema, ResourceSchema, RewardsSchema, SimpleItemSchema,
        SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, TaskType,
    },
    resources::RESOURCES,
    FightSimulator, HasDrops, ITEMS, MAPS, MONSTERS,
};
use itertools::Itertools;
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    cmp::min,
    option::Option,
    sync::{Arc, RwLock},
    vec::Vec,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIs;
use thiserror::Error;

#[derive(Default)]
pub struct Character {
    pub id: usize,
    pub inner: CharacterRequestHandler,
    pub inventory: Arc<Inventory>,
}

impl Character {
    pub fn new(id: usize, data: CharacterData) -> Self {
        Self {
            id,
            inner: CharacterRequestHandler::new(data.clone()),
            inventory: Arc::new(Inventory::new(data.clone())),
        }
    }

    pub fn run_loop(&self) {
        info!("{}: started !", self.inner.name());
        loop {
            if self.conf().read().unwrap().idle {
                continue;
            }
            if self.inventory.is_full() {
                self.deposit_all();
                continue;
            }
            MAPS.refresh_from_events();
            self.order_food();
            if self.handle_goals() {
                continue;
            }
            // TODO: improve fallback
            match self.progress_task() {
                Ok(_) => continue,
                Err(CharacterError::MissingItems { item, quantity }) => {
                    let _ = ORDER_BOARD.add(
                        Some(&self.inner.name()),
                        &item,
                        quantity,
                        Purpose::Task {
                            char: self.inner.name().to_owned(),
                        },
                    );
                    continue;
                }
                Err(_) => (),
            }
            for s in self.conf().read().unwrap().skills.iter() {
                if self.level_skill_up(*s) {
                    continue;
                }
            }
        }
    }

    fn handle_goals(&self) -> bool {
        let first_level_goal_not_reached = self
            .conf()
            .read()
            .unwrap()
            .goals
            .iter()
            .find(|g| {
                if let Goal::ReachSkillLevel { skill, level } = g {
                    self.skill_level(*skill) < *level
                } else {
                    false
                }
            })
            .cloned();
        // TODO: improve the way ReachSkillLevel is handled
        self.conf()
            .read()
            .unwrap()
            .goals
            .iter()
            .filter(|g| {
                g.is_reach_skill_level()
                    && first_level_goal_not_reached.is_some_and(|gnr| **g == gnr)
                    || !g.is_reach_skill_level()
            })
            .any(|g| match g {
                Goal::Events => false,
                Goal::Orders => self.handle_orderboard(),
                Goal::ReachSkillLevel { skill, level } if self.skill_level(*skill) < *level => {
                    self.level_skill_up(*skill)
                }
                Goal::FollowMaxSkillLevel {
                    skill,
                    skill_to_follow,
                } if self.skill_level(*skill)
                    < min(1 + ACCOUNT.max_skill_level(*skill_to_follow), MAX_LEVEL) =>
                {
                    self.level_skill_up(*skill)
                }
                _ => false,
            })
    }

    fn level_skill_up(&self, skill: Skill) -> bool {
        if self.skill_level(skill) >= MAX_LEVEL {
            return false;
        };
        if skill.is_combat() {
            return self.level_combat().is_ok();
        }
        self.level_skill_by_crafting(skill).is_ok() || self.level_skill_by_gathering(&skill).is_ok()
    }

    fn level_skill_by_gathering(&self, skill: &Skill) -> Result<(), CharacterError> {
        let Some(resource) = LEVELING_HELPER.best_resource(self.skill_level(*skill), *skill) else {
            return Err(CharacterError::ResourceNotFound);
        };
        self.gather_resource(&resource)?;
        Ok(())
    }

    fn level_skill_by_crafting(&self, skill: Skill) -> Result<(), CharacterError> {
        let Some(item) = LEVELING_HELPER.best_craft(self.skill_level(skill), skill, self) else {
            return Err(CharacterError::ItemNotFound);
        };
        let craft = self.craft_from_bank(
            &item.code,
            self.max_craftable_items(&item.code),
            if skill.is_gathering() || skill.is_cooking() {
                PostCraftAction::Deposit
            } else {
                PostCraftAction::Recycle
            },
        );
        if let Err(CharacterError::InsuffisientMaterials) = craft {
            if (!skill.is_gathering()
                || skill.is_alchemy()
                    && LEVELING_HELPER
                        .best_resource(self.skill_level(skill), skill)
                        .is_none())
                && self.order_missing_mats(
                    &item.code,
                    self.max_craftable_items(&item.code),
                    Purpose::Leveling {
                        char: self.inner.name().to_owned(),
                        skill,
                    },
                )
            {
                return Ok(());
            }
        };
        craft.map(|s| {
            info!(
                "{} crafted '{}'x{} to level up.",
                self.inner.name(),
                &item.code,
                s.amount_of(&item.code)
            );
            self.deposit_all();
        })
    }

    /// Browse orderboard for completable orders: first check if some orders
    /// can be turned in, then check for completable orders (enough materials to craft all items
    /// from an order. Then check for orders that can be progressed. Then check for order for which
    /// the skill level required needs to be leveled.
    fn handle_orderboard(&self) -> bool {
        let orders = ORDER_BOARD.orders_by_priority();
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
        ITEMS.best_source_of(&order.item).iter().any(|s| match s {
            ItemSource::Resource(r) => self.can_gather(r).is_ok(),
            ItemSource::Monster(m) => self.can_kill(m).is_ok(),
            ItemSource::Craft => self.can_craft(&order.item).is_ok(),
            ItemSource::TaskReward => order.in_progress() <= 0,
            ItemSource::Task => true,
            //ItemSource::Gift => true,
        })
    }

    /// Creates orders based on the missing (not available in bank) materials requiered to craft
    /// the `quantity` of the given `item`. Orders are created with the given `priority` and
    /// `purpose`. Returns true if an order has been made.
    fn order_missing_mats(&self, item: &str, quantity: i32, purpose: Purpose) -> bool {
        let mut ordered: bool = false;
        if quantity <= 0 {
            return false;
        }
        ITEMS
            .mats_of(item)
            .into_iter()
            .filter(|m| {
                BANK.has_available(&m.code, Some(&self.inner.name())) < m.quantity * quantity
            })
            .update(|m| {
                m.quantity = m.quantity * quantity
                    - if ORDER_BOARD.is_ordered(&m.code) {
                        0
                    } else {
                        BANK.has_available(&m.code, Some(&self.inner.name()))
                    }
            })
            .for_each(|m| {
                if ORDER_BOARD
                    .add(None, &m.code, m.quantity, purpose.clone())
                    .is_ok()
                {
                    ordered = true
                }
            });
        ordered
    }

    fn can_complete(&self, order: &Order) -> bool {
        ITEMS.best_source_of(&order.item).iter().any(|s| match s {
            ItemSource::Resource(_) => false,
            ItemSource::Monster(_) => false,
            ItemSource::Craft => {
                self.can_craft(&order.item).is_ok()
                    && BANK
                        .missing_mats_for(
                            &order.item,
                            ORDER_BOARD.total_missing_for(order),
                            Some(&self.inner.name()),
                        )
                        .is_empty()
            }
            ItemSource::TaskReward => {
                self.has_available(TASKS_COIN) >= TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
            }
            ItemSource::Task => self.has_available(&self.task()) >= self.task_missing(),
            //ItemSource::Gift => self.has_available(GIFT) > 0,
        })
    }

    fn handle_order(&self, order: Arc<Order>) -> bool {
        if ORDER_BOARD.total_missing_for(&order) <= 0 {
            return false;
        }
        let Some(progress) = self.progress_order(&order) else {
            return false;
        };
        if progress > 0 {
            info!(
                "{}: progressed by {} on order: {}, in inventories: {}",
                self.inner.name(),
                progress,
                order,
                ACCOUNT.available_in_inventories(&order.item),
            );
        }
        self.turn_in_order(order);
        true
    }

    fn progress_order(&self, order: &Order) -> Option<i32> {
        ITEMS
            .best_source_of(&order.item)
            .iter()
            .find_map(|s| match s {
                ItemSource::Resource(r) => self.progress_resource_order(order, r),
                ItemSource::Monster(m) => self.progress_monster_order(order, m),
                ItemSource::Craft => self.progress_crafting_order(order),
                ItemSource::TaskReward => self.progress_task_reward_order(order),
                ItemSource::Task => self.progress_task_order(order),
                //ItemSource::Gift => self.progress_gift_order(order),
            })
    }

    fn progress_resource_order(&self, order: &Order, r: &ResourceSchema) -> Option<i32> {
        order.inc_in_progress(1);
        let result = self
            .gather_resource(r)
            .ok()
            .map(|gather| gather.amount_of(&order.item));
        order.dec_in_progress(1);
        result
    }

    fn progress_monster_order(&self, order: &Order, m: &MonsterSchema) -> Option<i32> {
        self.kill_monster(m)
            .ok()
            .map(|fight| fight.amount_of(&order.item))
    }

    fn progress_crafting_order(&self, order: &Order) -> Option<i32> {
        if self.can_craft(&order.item).is_err() {
            return None;
        }
        if self.order_missing_mats(
            &order.item,
            ORDER_BOARD.total_missing_for(order),
            order.purpose.clone(),
        ) {
            return Some(0);
        }
        let quantity = min(
            self.max_craftable_items(&order.item),
            ORDER_BOARD.total_missing_for(order),
        );
        if quantity <= 0 {
            return None;
        }
        order.inc_in_progress(quantity);
        let crafted = self.craft_from_bank(&order.item, quantity, PostCraftAction::Keep);
        order.dec_in_progress(quantity);
        crafted.ok().map(|craft| craft.amount_of(&order.item))
    }

    fn progress_task_reward_order(&self, order: &Order) -> Option<i32> {
        match self.can_exchange_task() {
            Ok(()) => {
                order.inc_in_progress(1);
                let exchanged = self.exchange_task().map(|r| r.amount_of(&order.item)).ok();
                order.dec_in_progress(1);
                exchanged
            }
            Err(e) => {
                if ORDER_BOARD.total_missing_for(order) <= 0 {
                    return None;
                }
                if let CharacterError::NotEnoughCoin = e {
                    let q = TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
                        - if ORDER_BOARD.is_ordered(TASKS_COIN) {
                            0
                        } else {
                            self.has_in_bank_or_inv(TASKS_COIN)
                        };
                    return ORDER_BOARD
                        .add(None, TASKS_COIN, q, order.purpose.to_owned())
                        .ok()
                        .map(|_| 0);
                }
                None
            }
        }
    }

    fn progress_task_order(&self, order: &Order) -> Option<i32> {
        match self.complete_task() {
            Ok(r) => Some(r.amount_of(&order.item)),
            Err(e) => {
                if let CharacterError::NoTask = e {
                    let r#type = self.conf().read().unwrap().task_type;
                    if let Err(e) = self.accept_task(r#type) {
                        error!(
                            "{} error while accepting new task: {:?}",
                            self.inner.name(),
                            e
                        )
                    }
                    return Some(0);
                }
                let CharacterError::TaskNotFinished = e else {
                    return None;
                };
                match self.progress_task() {
                    Ok(_) => Some(0),
                    Err(CharacterError::MissingItems { item, quantity }) => ORDER_BOARD
                        .add(
                            Some(&self.inner.name()),
                            &item,
                            quantity,
                            order.purpose.clone(),
                        )
                        .ok()
                        .map(|_| 0),
                    _ => None,
                }
            }
        }
    }

    //fn progress_gift_order(&self, order: &Order) -> Option<i32> {
    //    match self.can_exchange_gift() {
    //        Ok(()) => {
    //            order.inc_in_progress(1);
    //            let exchanged = self.exchange_gift().map(|r| r.amount_of(&order.item)).ok();
    //            order.dec_in_progress(1);
    //            exchanged
    //        }
    //        Err(e) => {
    //            if ORDER_BOARD.total_missing_for(order) <= 0 {
    //                return None;
    //            }
    //            if let CharacterError::NotEnoughGift = e {
    //                let q = 1 - if ORDER_BOARD.is_ordered(GIFT) {
    //                    0
    //                } else {
    //                    self.has_in_bank_or_inv(GIFT)
    //                };
    //                return ORDER_BOARD
    //                    .add(None, GIFT, q, order.purpose.to_owned())
    //                    .ok()
    //                    .map(|_| 0);
    //            }
    //            None
    //        }
    //    }
    //}

    /// Deposit items requiered by the given `order` if needed.
    /// Returns true if items has be deposited.
    fn turn_in_order(&self, order: Arc<Order>) -> bool {
        if ORDER_BOARD.should_be_turned_in(&order) {
            return self.deposit_order(&order);
        }
        false
    }

    fn deposit_order(&self, order: &Order) -> bool {
        let q = self.inventory.has_available(&order.item);
        if q <= 0 {
            return false;
        }
        if self
            .deposit_item(
                &order.item,
                min(q, order.not_deposited()),
                order.owner.clone(),
            )
            .is_ok()
        {
            if let Err(e) = ORDER_BOARD.register_deposit(
                &order.owner,
                &order.item,
                min(q, order.not_deposited()),
                &order.purpose,
            ) {
                error!("{} failed to register deposit: {:?}", self.inner.name(), e);
            }
        }
        false
    }

    fn progress_task(&self) -> Result<(), CharacterError> {
        if self.task().is_empty() {
            let r#type = self.conf().read().unwrap().task_type;
            return self.accept_task(r#type).map(|_| ());
        }
        if self.task_finished() {
            return self.complete_task().map(|_| ());
        }
        let Some(monster) = MONSTERS.get(&self.task()) else {
            return self.trade_task().map(|_| ());
        };
        match self.kill_monster(&monster) {
            Ok(_) => Ok(()),
            Err(e) => {
                if let CharacterError::GearTooWeak { monster_code: _ } = e {
                    warn!("{}: {}", self.inner.name(), e);
                    self.cancel_task()?;
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn trade_task(&self) -> Result<TaskTradeSchema, CharacterError> {
        self.can_trade_task()?;
        let q = min(self.task_missing(), self.inventory.max_items());
        if let Err(e) = BANK.reserv(&self.task(), q, &self.inner.name()) {
            error!(
                "{}: error while reserving items for item task: {:?}",
                self.inner.name(),
                e
            )
        }
        self.deposit_all();
        if let Err(e) = self.withdraw_item(&self.task(), q) {
            error!("{}: error while withdrawing {:?}", self.inner.name(), e);
            BANK.decrease_reservation(&self.task(), q, &self.inner.name());
        };
        if let Err(e) = self.move_to_closest_taskmaster(self.task_type()) {
            error!(
                "{}: error while moving to taskmaster: {:?}",
                self.inner.name(),
                e
            );
        };
        let res = self.inner.request_task_trade(&self.task(), q);
        self.inventory.decrease_reservation(&self.task(), q);
        Ok(res?)
    }

    fn can_trade_task(&self) -> Result<(), CharacterError> {
        if self.task().is_empty() {
            return Err(CharacterError::NoTask);
        }
        if self.task_type().is_none_or(|tt| tt != TaskType::Items) {
            return Err(CharacterError::InvalidTaskType);
        }
        if self.task_missing() <= 0 {
            return Err(CharacterError::TaskAlreadyCompleted);
        }
        if self.task_missing()
            > BANK.has_available(&self.task(), Some(&self.inner.name()))
                + self.inventory.total_of(&self.task())
        {
            return Err(CharacterError::MissingItems {
                item: self.task().to_owned(),
                quantity: self.task_missing()
                    - BANK.has_available(&self.task(), Some(&self.inner.name()))
                    - self.inventory.total_of(&self.task()),
            });
        }
        Ok(())
    }

    fn accept_task(&self, r#type: TaskType) -> Result<TaskSchema, CharacterError> {
        self.move_to_closest_taskmaster(Some(r#type))?;
        Ok(self.inner.request_accept_task()?)
    }

    fn complete_task(&self) -> Result<RewardsSchema, CharacterError> {
        if self.task().is_empty() {
            return Err(CharacterError::NoTask);
        }
        if !self.task_finished() {
            return Err(CharacterError::TaskNotFinished);
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        self.inner.request_complete_task().map_err(|e| e.into())
    }

    fn can_exchange_task(&self) -> Result<(), CharacterError> {
        if self.inventory.total_of(TASKS_COIN)
            + BANK.has_available(TASKS_COIN, Some(&self.inner.name()))
            < TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
        {
            return Err(CharacterError::NotEnoughCoin);
        }
        Ok(())
    }

    fn exchange_task(&self) -> Result<RewardsSchema, CharacterError> {
        self.can_exchange_task()?;
        let mut quantity = min(
            self.inventory.max_items() / 2,
            BANK.has_available(TASKS_COIN, Some(&self.inner.name())),
        );
        quantity = quantity - (quantity % TASK_EXCHANGE_PRICE);
        if self.inventory.total_of(TASKS_COIN) >= TASK_EXCHANGE_PRICE {
            if let Err(e) = self
                .inventory
                .reserv(TASKS_COIN, self.inventory.total_of(TASKS_COIN))
            {
                error!(
                    "{}: error while reserving tasks coins in inventory: {}",
                    self.inner.name(),
                    e
                );
            }
        } else {
            if BANK
                .reserv(TASKS_COIN, quantity, &self.inner.name())
                .is_err()
            {
                return Err(CharacterError::NotEnoughCoin);
            }
            self.deposit_all();
            self.withdraw_item(TASKS_COIN, quantity)?;
        }
        if let Err(e) = self.move_to_closest_taskmaster(self.task_type()) {
            error!(
                "{}: error while moving to taskmaster: {:?}",
                self.inner.name(),
                e
            );
        };
        let result = self.inner.request_task_exchange().map_err(|e| e.into());
        self.inventory
            .decrease_reservation(TASKS_COIN, TASK_EXCHANGE_PRICE);
        result
    }

    //fn can_exchange_gift(&self) -> Result<(), CharacterError> {
    //    if self.inventory.total_of(GIFT) + BANK.has_available(GIFT, Some(&self.inner.name())) < 1 {
    //        return Err(CharacterError::NotEnoughGift);
    //    }
    //    Ok(())
    //}

    //fn exchange_gift(&self) -> Result<RewardsSchema, CharacterError> {
    //    self.can_exchange_gift()?;
    //    let quantity = min(
    //        self.inventory.max_items() / 2,
    //        BANK.has_available(GIFT, Some(&self.inner.name())),
    //    );
    //    if self.inventory.total_of(GIFT) >= 1 {
    //        if let Err(e) = self.inventory.reserv(GIFT, self.inventory.total_of(GIFT)) {
    //            error!(
    //                "{}: error while reserving gift in inventory: {}",
    //                self.inner.name(),
    //                e
    //            );
    //        }
    //    } else {
    //        if BANK.reserv(GIFT, quantity, &self.inner.name()).is_err() {
    //            return Err(CharacterError::NotEnoughGift);
    //        }
    //        self.deposit_all();
    //        self.withdraw_item(GIFT, quantity)?;
    //    }
    //    if let Err(e) = self.move_to_closest_map_of_type(ContentType::SantaClaus) {
    //        error!(
    //            "{}: error while moving to santa claus: {:?}",
    //            self.inner.name(),
    //            e
    //        );
    //    };
    //    let result = self.inner.request_gift_exchange().map_err(|e| e.into());
    //    self.inventory.decrease_reservation(GIFT, 1);
    //    result
    //}

    fn cancel_task(&self) -> Result<(), CharacterError> {
        if BANK.has_available(TASKS_COIN, Some(&self.inner.name()))
            < TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
        {
            return Err(CharacterError::NotEnoughCoin);
        }
        if self.inventory.has_available(TASKS_COIN) <= 0 {
            if BANK
                .reserv("tasks_coin", TASK_CANCEL_PRICE, &self.inner.name())
                .is_err()
            {
                return Err(CharacterError::NotEnoughCoin);
            }
            self.deposit_all();
            self.withdraw_item(TASKS_COIN, TASK_CANCEL_PRICE)?;
        }
        if let Err(e) = self.move_to_closest_taskmaster(self.task_type()) {
            error!(
                "{}: error while moving to taskmaster: {:?}",
                self.inner.name(),
                e
            );
        };
        let result = self.inner.request_cancel_task().map_err(|e| e.into());
        self.inventory
            .decrease_reservation(TASKS_COIN, TASK_CANCEL_PRICE);
        result
    }

    /// Find a target and kill it if possible.
    fn level_combat(&self) -> Result<(), CharacterError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(CharacterError::SkillDisabled(Skill::Combat));
        }
        if let Ok(_) | Err(CharacterError::NoTask) = self.complete_task() {
            if let Err(e) = self.accept_task(TaskType::Monsters) {
                error!(
                    "{} error while accepting new task: {:?}",
                    self.inner.name(),
                    e
                )
            }
        }
        if self.task_type().is_some_and(|t| t == TaskType::Monsters) && self.progress_task().is_ok()
        {
            return Ok(());
        }
        let Some(monster) = LEVELING_HELPER.best_monster(self) else {
            return Err(CharacterError::MonsterNotFound);
        };
        self.kill_monster(&monster)?;
        Ok(())
    }

    fn move_to(&self, x: i32, y: i32) -> Result<Arc<MapSchema>, CharacterError> {
        if self.inner.position() == (x, y) {
            return Ok(self.map());
        }
        Ok(self.inner.request_move(x, y).map(|_| self.map())?)
    }

    /// Checks if an gear making the `Character` able to kill the given
    /// `monster` is available, equip it, then move the `Character` to the given
    /// map or the closest containing the `monster` and fight it.
    fn kill_monster(&self, monster: &MonsterSchema) -> Result<FightSchema, CharacterError> {
        self.can_fight(monster)?;
        self.check_gear(monster)?;
        if let Ok(_) | Err(CharacterError::NoTask) = self.complete_task() {
            if let Err(e) = self.accept_task(TaskType::Monsters) {
                error!(
                    "{} error while accepting new task: {:?}",
                    self.inner.name(),
                    e
                )
            }
        }
        self.withdraw_food();
        if !self.can_kill_now(monster) {
            self.eat_food();
        }
        if !self.can_kill_now(monster) {
            if let Err(e) = self.rest() {
                error!("{} failed to rest: {:?}", self.inner.name(), e)
            }
        }
        self.move_to_closest_map_with_content_code(&monster.code)?;
        Ok(self.inner.request_fight()?)
    }

    fn check_gear(&self, monster: &MonsterSchema) -> Result<(), CharacterError> {
        let mut available: Gear;
        let Ok(_browsed) = BANK.browsed.write() else {
            return Err(CharacterError::BankUnavailable);
        };
        match self.can_kill(monster) {
            Ok(gear) => {
                available = gear;
                self.reserv_gear(&mut available)
            }
            Err(e) => return Err(e),
        }
        self.order_best_gear_against(monster);
        drop(_browsed);
        self.equip_gear(&mut available);
        Ok(())
    }

    fn rest(&self) -> Result<(), CharacterError> {
        if self.health() < self.max_health() {
            self.inner.request_rest()?;
        }
        Ok(())
    }

    /// Checks if the character is able to gather the given `resource`. If it
    /// can, equips the best available appropriate tool, then move the `Character`
    /// to the given map or the closest containing the `resource` and gather it.  
    fn gather_resource(
        &self,
        resource: &ResourceSchema,
    ) -> Result<SkillDataSchema, CharacterError> {
        self.can_gather(resource)?;
        let Some(map) = self.map().closest_with_content_code(&resource.code) else {
            return Err(CharacterError::MapNotFound);
        };
        self.check_for_tool(resource);
        self.move_to(map.x, map.y)?;
        Ok(self.inner.request_gather()?)
    }

    fn check_for_tool(&self, resource: &ResourceSchema) {
        let mut available: Option<Arc<ItemSchema>> = None;
        let prev_equiped = self.equiped_in(Slot::Weapon);
        if let Ok(_browsed) = BANK.browsed.write() {
            if let Some(ref tool) = GEAR_FINDER.best_tool(
                self,
                resource.skill.into(),
                Filter {
                    available: true,
                    ..Default::default()
                },
            ) {
                available = Some((*tool).clone());
                self.reserv_if_needed_and_available(Slot::Weapon, &tool.code, 1);
            }
            self.order_best_tool(resource.skill.into());
        }
        if let Some(prev_equiped) = prev_equiped {
            if available.is_none()
                || available
                    .as_ref()
                    .is_some_and(|t| t.code != prev_equiped.code)
            {
                if let Err(e) = self.unequip_item(Slot::Weapon, 1) {
                    error!(
                        "{}: failed to unequip previously equiped weapon: {:?}",
                        self.inner.name(),
                        e
                    )
                }
                // TODO: improve logic: maybe include this logic in `deposit_item` method
                if let Some(o) = ORDER_BOARD
                    .orders_by_priority()
                    .iter()
                    .find(|o| o.item == prev_equiped.code)
                {
                    self.deposit_order(o);
                } else if let Err(e) = self.deposit_item(&prev_equiped.code, 1, None) {
                    error!(
                        "{}: error while depositing previously equiped weapon: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
        }
        self.unequip_and_deposit_all_for_gathering();
        if let Some(available) = available {
            self.equip_item_from_bank_or_inventory(&available.code, Slot::Weapon);
        }
    }

    fn order_best_tool(&self, skill: Skill) {
        if let Some(best) = GEAR_FINDER.best_tool(
            self,
            skill,
            Filter {
                can_craft: true,
                ..Default::default()
            },
        ) {
            self.order_if_needed(Slot::Weapon, &best.code, 1);
        }
    }

    pub fn time_to_kill(&self, monster: &MonsterSchema) -> Option<i32> {
        match self.can_kill(monster) {
            Ok(gear) => {
                let fight = FIGHT_SIMULATOR.simulate(self.level(), 0, &gear, monster, false);
                Some(fight.cd + (fight.hp_lost / 5 + if fight.hp_lost % 5 > 0 { 1 } else { 0 }))
            }
            Err(_) => None,
        }
    }

    pub fn time_to_gather(&self, resource: &ResourceSchema) -> Option<i32> {
        if self.can_gather(resource).is_err() {
            return None;
        }
        let tool = self.best_tool_for_resource(&resource.code);
        let time = FIGHT_SIMULATOR.gather(
            self.skill_level(resource.skill.into()),
            resource.level,
            tool.map_or(0, |t| t.skill_cooldown_reduction(resource.skill.into())),
        );
        Some(time)
    }

    #[allow(dead_code)]
    pub fn time_to_get(&self, item: &str) -> Option<i32> {
        ITEMS
            .best_source_of(item)
            .iter()
            .filter_map(|s| match s {
                ItemSource::Resource(r) => self.time_to_gather(r),
                ItemSource::Monster(m) => self
                    .time_to_kill(m)
                    .map(|time| time * ITEMS.drop_rate(item)),
                ItemSource::Craft => Some(
                    CRAFT_TIME
                        + ITEMS
                            .mats_of(item)
                            .iter()
                            .map(|m| self.time_to_get(&m.code).unwrap_or(1000) * m.quantity)
                            .sum::<i32>(),
                ),
                ItemSource::TaskReward => Some(2000),
                ItemSource::Task => Some(2000),
                //ItemSource::Gift => Some(1000),
            })
            .min()
    }

    pub fn can_fight(&self, monster: &MonsterSchema) -> Result<(), CharacterError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(CharacterError::SkillDisabled(Skill::Combat));
        }
        if MAPS.with_content_code(&monster.code).is_empty() {
            return Err(CharacterError::MapNotFound);
        }
        if self.inventory.is_full() {
            return Err(CharacterError::InventoryFull);
        }
        Ok(())
    }

    /// Checks if the `Character` is able to kill the given monster and returns
    /// the best available gear to do so.
    pub fn can_kill<'a>(&'a self, monster: &'a MonsterSchema) -> Result<Gear, CharacterError> {
        self.can_fight(monster)?;
        let available = GEAR_FINDER.best_winning_against(
            self,
            monster,
            Filter {
                available: true,
                ..Default::default()
            },
        );
        if self.can_kill_with(monster, &available) {
            Ok(available)
        } else {
            Err(CharacterError::GearTooWeak {
                monster_code: monster.code.to_owned(),
            })
        }
    }

    /// Checks if the `Character` could kill the given `monster` with the given
    /// `gear`
    fn can_kill_with(&self, monster: &MonsterSchema, gear: &Gear) -> bool {
        FIGHT_SIMULATOR
            .simulate(self.inner.level(), 0, gear, monster, false)
            .result
            == FightResult::Win
    }

    fn can_kill_now(&self, monster: &MonsterSchema) -> bool {
        FIGHT_SIMULATOR
            .simulate(
                self.inner.level(),
                self.inner.missing_hp(),
                &self.gear(),
                monster,
                false,
            )
            .result
            == FightResult::Win
    }

    // Checks that the `Character` has the required skill level to gather the given `resource`
    fn can_gather(&self, resource: &ResourceSchema) -> Result<(), CharacterError> {
        let skill: Skill = resource.skill.into();
        if !self.skill_enabled(skill) {
            return Err(CharacterError::SkillDisabled(skill));
        }
        if self.inner.skill_level(skill) < resource.level {
            return Err(CharacterError::InsuffisientSkillLevel(
                skill,
                resource.level,
            ));
        }
        if self.inventory.is_full() {
            return Err(CharacterError::InventoryFull);
        }
        Ok(())
    }

    // Checks that the `Character` has the required skill level to craft the given item `code`
    pub fn can_craft(&self, item: &str) -> Result<(), CharacterError> {
        let Some(item) = ITEMS.get(item) else {
            return Err(CharacterError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CharacterError::ItemNotCraftable);
        };
        if !self.skill_enabled(skill) {
            return Err(CharacterError::SkillDisabled(skill));
        }
        if self.inner.skill_level(skill) < item.level {
            return Err(CharacterError::InsuffisientSkillLevel(skill, item.level));
        }
        // TODO: improve condition
        if self.inventory.is_full() {
            return Err(CharacterError::InventoryFull);
        }
        Ok(())
    }

    pub fn can_recycle(&self, item: &str, quantity: i32) -> Result<(), CharacterError> {
        let Some(item) = ITEMS.get(item) else {
            return Err(CharacterError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CharacterError::ItemNotCraftable);
        };
        if !self.skill_enabled(skill) {
            return Err(CharacterError::SkillDisabled(skill));
        };
        if self.inner.skill_level(skill) < item.level {
            return Err(CharacterError::InsuffisientSkillLevel(skill, item.level));
        };
        if self.inventory.max_items() < item.recycled_quantity() * quantity {
            return Err(CharacterError::InsuffisientInventorySpace);
        }
        Ok(())
    }

    /// Crafts the given `quantity` of the given item `code` if the required
    /// materials to craft them in one go are available in bank and deposit the crafted
    /// items into the bank.
    pub fn craft_from_bank(
        &self,
        item: &str,
        quantity: i32,
        post_action: PostCraftAction,
    ) -> Result<SkillInfoSchema, CharacterError> {
        self.can_craft(item)?;
        if self.max_craftable_items_from_bank(item) < quantity {
            return Err(CharacterError::InsuffisientMaterials);
        }
        info!(
            "{}: going to craft '{}'x{} from bank.",
            self.inner.name(),
            item,
            quantity
        );
        ITEMS.mats_of(item).iter().for_each(|m| {
            if let Err(e) = BANK.reserv(&m.code, m.quantity * quantity, &self.inner.name()) {
                error!(
                    "{}: error while reserving mats for crafting from bank: {:?}",
                    self.inner.name(),
                    e
                )
            }
        });
        self.deposit_all();
        let mats = self.withdraw_mats_for(item, quantity)?;
        if let Err(e) = self.move_to_craft(item) {
            error!(
                "{}: error while moving to craft: {:?}",
                self.inner.name(),
                e
            );
        };
        let craft = self.inner.request_craft(item, quantity);
        mats.iter().for_each(|m| {
            self.inventory.decrease_reservation(&m.code, m.quantity);
        });
        match post_action {
            PostCraftAction::Deposit => {
                if let Err(e) = self.deposit_item(item, quantity, None) {
                    error!(
                        "{}: error while depositing items after crafting from bank: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
            PostCraftAction::Recycle => {
                if let Err(e) = self.recycle_item(item, quantity) {
                    error!(
                        "{}: error while recycling items after crafting from bank: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
            PostCraftAction::Keep => (),
        };
        Ok(craft?)
    }

    pub fn recycle_item(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<RecyclingItemsSchema, CharacterError> {
        self.can_recycle(item, quantity)?;
        let quantity_available =
            self.inventory.total_of(item) + BANK.has_available(item, Some(&self.inner.name()));
        if quantity_available < quantity {
            return Err(CharacterError::QuantityUnavailable(quantity));
        }
        info!(
            "{}: going to recycle '{}x{}'.",
            self.inner.name(),
            item,
            quantity
        );
        if self.inventory.total_of(item) < quantity {
            let missing_quantity = quantity - self.inventory.has_available(item);
            if let Err(e) = BANK.reserv(item, missing_quantity, &self.inner.name()) {
                error!(
                    "{}: error while reserving '{}': {:?}",
                    self.inner.name(),
                    item,
                    e
                );
            }
            self.deposit_all();
            self.withdraw_item(item, missing_quantity)?;
        }
        self.move_to_craft(item)?;
        let result = self.inner.request_recycle(item, quantity);
        self.inventory
            .decrease_reservation(&self.inner.task(), quantity);
        Ok(result?)
    }

    pub fn delete_item(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, CharacterError> {
        let quantity_available =
            self.inventory.has_available(item) + BANK.has_available(item, Some(&self.inner.name()));
        if quantity_available < quantity {
            return Err(CharacterError::QuantityUnavailable(quantity));
        }
        info!(
            "{}: going to delete '{}x{}'.",
            self.inner.name(),
            item,
            quantity
        );
        if self.inventory.has_available(item) < quantity {
            let missing_quantity = quantity - self.inventory.has_available(item);
            if let Err(e) = BANK.reserv(item, missing_quantity, &self.inner.name()) {
                error!(
                    "{}: error while reserving '{}': {:?}",
                    self.inner.name(),
                    item,
                    e
                );
            }
            self.deposit_all();
            self.withdraw_item(item, missing_quantity)?;
        }
        let result = self.inner.request_delete(item, quantity);
        self.inventory
            .decrease_reservation(&self.inner.task(), quantity);
        Ok(result?)
    }

    pub fn deposit_item(
        &self,
        item: &str,
        quantity: i32,
        owner: Option<String>,
    ) -> Result<SimpleItemSchema, CharacterError> {
        if self.inventory.total_of(item) < quantity {
            // TODO: return a better error
            return Err(CharacterError::ItemNotFound);
        }
        self.move_to_closest_map_of_type(ContentType::Bank)?;
        if BANK.free_slots() <= BANK_MIN_FREE_SLOT {
            if let Err(e) = self.expand_bank() {
                error!(
                    "{}: failed to expand bank capacity: {:?}",
                    self.inner.name(),
                    e
                )
            }
        }
        let deposit = self.inner.request_deposit(item, quantity);
        if deposit.is_ok() {
            if let Some(owner) = owner {
                if let Err(e) = BANK.increase_reservation(item, quantity, &owner) {
                    error!(
                        "{}: failed to reserv deposited item: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
            self.inventory.decrease_reservation(item, quantity);
        }
        if let Err(e) = self.deposit_all_gold() {
            error!(
                "{}: failed to deposit gold to the bank: {:?}",
                self.inner.name(),
                e
            )
        }
        Ok(deposit?)
    }

    pub fn withdraw_item(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, CharacterError> {
        if BANK.has_available(item, Some(&self.inner.name())) < quantity {
            // TODO: return a better error
            return Err(CharacterError::ItemNotFound);
        }
        self.move_to_closest_map_of_type(ContentType::Bank)?;
        let result = self.inner.request_withdraw(item, quantity);
        if result.is_ok() {
            BANK.decrease_reservation(item, quantity, &self.inner.name());
            if let Err(e) = self.inventory.reserv(item, quantity) {
                error!(
                    "{}: failed to reserv withdrawed item '{}'x{}: {:?}",
                    self.inner.name(),
                    item,
                    quantity,
                    e
                );
            }
        }
        Ok(result?)
    }

    /// Deposits all the gold and items in the character inventory into the bank.
    /// Items needed by orders are turned in first.
    /// Bank is expanded if close to being full.
    /// TODO: add returns type with Result breakdown
    pub fn deposit_all(&self) {
        if self.inventory.total_items() <= 0 {
            return;
        }
        info!(
            "{}: going to deposit all items to the bank.",
            self.inner.name(),
        );
        ORDER_BOARD.orders_by_priority().iter().for_each(|o| {
            self.deposit_order(o);
        });
        self.inventory.copy().iter().for_each(|slot| {
            if slot.quantity > 0 {
                if let Err(e) = self.deposit_item(&slot.code, slot.quantity, None) {
                    error!(
                        "{}: error while depositing all to bank: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
        });
    }

    pub fn deposit_all_but(&self, item: &str) {
        if self.inventory.total_items() <= 0 {
            return;
        }
        info!(
            "{}: going to deposit all items but '{item}' to the bank.",
            self.inner.name(),
        );
        ORDER_BOARD.orders_by_priority().iter().for_each(|o| {
            self.deposit_order(o);
        });
        self.inventory.copy().iter().for_each(|slot| {
            if slot.quantity > 0 && slot.code != item {
                if let Err(e) = self.deposit_item(&slot.code, slot.quantity, None) {
                    error!(
                        "{}: error while depositing all to bank: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
        });
    }

    pub fn deposit_all_gold(&self) -> Result<i32, CharacterError> {
        self.deposit_gold(self.gold())
    }

    pub fn deposit_gold(&self, amount: i32) -> Result<i32, CharacterError> {
        if amount <= 0 {
            return Ok(0);
        };
        if amount > self.gold() {
            return Err(CharacterError::InsuffisientGoldInInventory);
        }
        self.move_to_closest_map_of_type(ContentType::Bank)?;
        Ok(self.inner.request_deposit_gold(amount)?)
    }

    pub fn expand_bank(&self) -> Result<i32, CharacterError> {
        let Ok(_being_expanded) = BANK.being_expanded.try_write() else {
            return Err(CharacterError::BankUnavailable);
        };
        if BANK.gold() + self.gold() < BANK.next_expansion_cost() {
            return Err(CharacterError::InsuffisientGold);
        };
        self.withdraw_gold(BANK.next_expansion_cost() - self.gold())?;
        self.move_to_closest_map_of_type(ContentType::Bank)?;
        Ok(self.inner.request_expand_bank()?)
    }

    pub fn withdraw_gold(&self, amount: i32) -> Result<i32, CharacterError> {
        if amount <= 0 {
            return Ok(0);
        };
        if BANK.gold() < amount {
            return Err(CharacterError::InsuffisientGoldInBank);
        };
        self.move_to_closest_map_of_type(ContentType::Bank)?;
        Ok(self.inner.request_withdraw_gold(amount)?)
    }

    pub fn empty_bank(&self) {
        if let Err(e) = self.move_to_closest_map_of_type(ContentType::Bank) {
            error!(
                "{} failed to move to bank before emptying bank: {:?}",
                self.inner.name(),
                e
            )
        }
        self.deposit_all();
        BANK.content().iter().for_each(|i| {
            info!("{} deleting {:?}", self.inner.name(), i);
            let mut remain = i.quantity;
            while remain > 0 {
                let quantity = min(self.inventory.free_space(), remain);
                if let Err(e) = self.withdraw_item(&i.code, quantity) {
                    error!(
                        "{} error while withdrawing item during bank empting: {:?}",
                        self.inner.name(),
                        e
                    )
                }
                if let Err(e) = self.inner.request_delete(&i.code, quantity) {
                    error!(
                        "{} error while delete item during bank emptying: {:?}",
                        self.inner.name(),
                        e
                    )
                }
                self.inventory.decrease_reservation(&i.code, quantity);
                remain -= quantity;
            }
        });
    }

    /// Withdraw the materials required to craft the `quantity` of the
    /// item `code` and returns the maximum amount that can be crafted.
    // TODO: add check on `inventory_max_items`
    fn withdraw_mats_for(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<Vec<SimpleItemSchema>, CharacterError> {
        let mats = ITEMS
            .mats_of(item)
            .into_iter()
            .update(|m| m.quantity *= quantity)
            .collect_vec();
        for mat in &mats {
            if BANK.has_available(&mat.code, Some(&self.inner.name())) < mat.quantity {
                warn!("{}: not enough materials in bank to withdraw the materials required to craft '{item}'x{quantity}", self.inner.name());
                return Err(CharacterError::InsuffisientMaterials);
            }
        }
        info!(
            "{}: going to withdraw materials for '{item}'x{quantity}.",
            self.inner.name()
        );
        for mat in &mats {
            self.withdraw_item(&mat.code, mat.quantity)?;
        }
        Ok(mats)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on
    /// inventory max items
    pub fn max_craftable_items(&self, item: &str) -> i32 {
        self.inventory.max_items() / ITEMS.mats_quantity_for(item)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory max items and bank materials.
    fn max_craftable_items_from_bank(&self, item: &str) -> i32 {
        min(
            BANK.has_mats_for(item, Some(&self.inner.name())),
            self.inventory.max_items() / ITEMS.mats_quantity_for(item),
        )
    }

    /// Reycle the maximum amount of the item `code` with the items  currently
    /// available in the character inventory and returns the amount recycled.
    pub fn recycle_all(&self, item: &str) -> i32 {
        let n = self.inventory.total_of(item);
        if n > 0 {
            info!("{}: recycling all '{}'.", self.inner.name(), item);
            if let Err(e) = self.inner.request_recycle(item, n) {
                error!(
                    "{}: error while recycling all '{}': {:?}",
                    self.inner.name(),
                    item,
                    e
                )
            }
        }
        n
    }

    fn move_to_closest_map_of_type(
        &self,
        r#type: ContentType,
    ) -> Result<Arc<MapSchema>, CharacterError> {
        let Some(map) = self.map().closest_of_type(r#type) else {
            return Err(CharacterError::MapNotFound);
        };
        self.move_to(map.x, map.y)
    }

    fn move_to_closest_taskmaster(
        &self,
        r#type: Option<TaskType>,
    ) -> Result<Arc<MapSchema>, CharacterError> {
        let Some(map) = self.map().closest_tasksmaster(r#type) else {
            return Err(CharacterError::MapNotFound);
        };
        self.move_to(map.x, map.y)
    }

    fn move_to_closest_map_with_content_code(
        &self,
        code: &str,
    ) -> Result<Arc<MapSchema>, CharacterError> {
        let Some(map) = self.map().closest_with_content_code(code) else {
            return Err(CharacterError::MapNotFound);
        };
        self.move_to(map.x, map.y)
    }

    /// Moves the `Character` to the crafting station corresponding to the skill
    /// required to craft the given item `code`.
    fn move_to_craft(&self, item: &str) -> Result<(), CharacterError> {
        let Some(skill) = ITEMS.get(item).and_then(|i| i.skill_to_craft()) else {
            return Err(CharacterError::ItemNotCraftable);
        };
        let Some(dest) = MAPS.with_workshop_for(skill) else {
            return Err(CharacterError::MapNotFound);
        };
        self.move_to(dest.x, dest.y)?;
        Ok(())
    }

    fn equip_gear(&self, gear: &mut Gear) {
        gear.align_to(&self.gear());
        Slot::iter().for_each(|s| {
            if let Some(item) = gear.slot(s) {
                self.equip_item_from_bank_or_inventory(&item.code, s);
            }
        });
    }

    fn equip_item(&self, item: &str, slot: Slot, quantity: i32) -> Result<(), CharacterError> {
        if let Some(item) = ITEMS.get(item) {
            if self.inventory.free_space() + item.inventory_space() <= 0 {
                self.deposit_all_but(&item.code);
            }
        }
        self.unequip_item(slot, self.quantity_in_slot(slot))?;
        if let Err(e) = self.inner.request_equip(item, slot, quantity) {
            error!(
                "{}: failed to equip '{}'x{} in the '{:?}' slot: {:?}",
                self.inner.name(),
                item,
                quantity,
                slot,
                e
            );
        }
        self.inventory.decrease_reservation(item, quantity);
        Ok(())
    }

    fn unequip_item(&self, slot: Slot, quantity: i32) -> Result<(), CharacterError> {
        // TODO: add check for inventory space
        let Some(equiped) = self.equiped_in(slot) else {
            return Ok(());
        };
        if equiped.health() >= self.inner.health() {
            self.eat_food();
        }
        if equiped.health() >= self.inner.health() {
            self.rest()?;
        }
        Ok(self.inner.request_unequip(slot, quantity)?)
    }

    fn equip_item_from_bank_or_inventory(&self, item: &str, slot: Slot) {
        let prev_equiped = self.equiped_in(slot);
        if prev_equiped.as_ref().is_some_and(|e| e.code == item) {
            return;
        }
        if self.inventory.total_of(item) <= 0
            && BANK.has_available(item, Some(&self.inner.name())) > 0
        {
            let q = min(
                slot.max_quantity(),
                BANK.has_available(item, Some(&self.inner.name())),
            );
            if self.inventory.free_space() < q {
                self.deposit_all();
            }
            if let Err(e) = self.withdraw_item(item, q) {
                error!(
                    "{} failed withdraw item from bank or inventory: {:?}",
                    self.inner.name(),
                    e
                );
            }
        }
        if let Err(e) = self.equip_item(
            item,
            slot,
            min(slot.max_quantity(), self.inventory.total_of(item)),
        ) {
            error!(
                "{} failed to equip item from bank or inventory: {:?}",
                self.inner.name(),
                e
            );
        }
        if let Some(i) = prev_equiped {
            // TODO: improve logic
            if let Some(o) = ORDER_BOARD
                .orders_by_priority()
                .iter()
                .find(|o| o.item == i.code)
            {
                self.deposit_order(o);
            }
            if self.inventory.total_of(&i.code) > 0 {
                if let Err(e) = self.deposit_item(&i.code, self.inventory.total_of(&i.code), None) {
                    error!(
                        "{} failed to deposit previously equiped item: {:?}",
                        self.inner.name(),
                        e
                    );
                }
            }
        }
    }

    fn best_tool_for_resource(&self, item: &str) -> Option<Arc<ItemSchema>> {
        match RESOURCES.get(item) {
            //TODO improve filtering
            Some(resource) => ITEMS
                .equipable_at_level(self.level(), Type::Weapon)
                .into_iter()
                .filter(|i| i.skill_cooldown_reduction(resource.skill.into()) < 0)
                .min_by_key(|i| i.skill_cooldown_reduction(Skill::from(resource.skill))),
            None => None,
        }
    }

    /// Returns the amount of the given item `code` available in bank and inventory.
    fn has_in_bank_or_inv(&self, item: &str) -> i32 {
        BANK.has_available(item, Some(&self.inner.name())) + self.inventory.total_of(item)
    }

    /// Returns the amount of the given item `code` available in bank, inventory and gear.
    pub fn has_available(&self, item: &str) -> i32 {
        self.has_in_bank_or_inv(item) + self.has_equiped(item) as i32
    }

    /// Checks if the given item `code` is equiped.
    fn has_equiped(&self, item: &str) -> usize {
        Slot::iter()
            .filter(|s| self.equiped_in(*s).is_some_and(|e| e.code == item))
            .count()
    }

    pub fn toggle_idle(&self) {
        let mut conf = self.conf().write().unwrap();
        conf.idle ^= true;
        info!("{} toggled idle: {}.", self.inner.name(), conf.idle);
        if !conf.idle {
            self.inner.refresh_data()
        }
    }

    pub fn conf(&self) -> &RwLock<CharConfig> {
        BOT_CONFIG.characters.get(self.id).unwrap()
    }

    #[allow(dead_code)]
    fn time_to_get_gear(&self, gear: &Gear) -> Option<i32> {
        Slot::iter()
            .map(|s| gear.slot(s).and_then(|i| ITEMS.time_to_get(&i.code)))
            .sum()
    }

    ////TODO: finish implementing this function
    //#[allow(dead_code)]
    //#[allow(unused_variables)]
    //fn order_upgrades(&self, current: Gear, monster: &MonsterSchema, filter: Filter) {
    //    let gears = GEAR_FINDER.bests_against(self, monster, filter);
    //    if let Some(gear) = gears
    //        .iter()
    //        .filter(|g| self.can_kill_with(monster, g))
    //        .min_by_key(|g| self.time_to_get_gear(g))
    //    {
    //        if self.can_kill_with(monster, gear) {
    //            self.order_gear(*gear);
    //        };
    //    }
    //}

    fn order_best_gear_against(&self, monster: &MonsterSchema) {
        let mut gear = GEAR_FINDER.best_winning_against(
            self,
            monster,
            Filter {
                can_craft: true,
                from_task: false,
                from_monster: false,
                ..Default::default()
            },
        );
        if self.can_kill_with(monster, &gear) {
            self.order_gear(&mut gear);
        };
    }

    fn order_gear(&self, gear: &mut Gear) {
        gear.align_to(&self.gear());
        Slot::iter().for_each(|s| {
            if !s.is_artifact_1()
                && !s.is_artifact_2()
                && !s.is_artifact_3()
                && !s.is_ring_1()
                && !s.is_ring_2()
            {
                if let Some(item) = gear.slot(s) {
                    let quantity = if s.is_utility_1() || s.is_utility_2() {
                        100
                    } else {
                        1
                    };
                    self.order_if_needed(s, &item.code, quantity);
                }
            }
        });
        if gear.ring1.is_some() && gear.ring1 == gear.ring2 {
            self.order_if_needed(Slot::Ring1, &gear.ring1.as_ref().unwrap().code, 2);
        } else {
            if let Some(ref ring1) = gear.ring1 {
                self.order_if_needed(Slot::Ring1, &ring1.code, 1);
            }
            if let Some(ref ring2) = gear.ring1 {
                self.order_if_needed(Slot::Ring2, &ring2.code, 1);
            }
        }
    }

    fn order_if_needed(&self, slot: Slot, item: &str, quantity: i32) -> bool {
        if (self.equiped_in(slot).is_none()
            || self
                .equiped_in(slot)
                .is_some_and(|equiped| item != equiped.code))
            && self.has_in_bank_or_inv(item) < quantity
        {
            return ORDER_BOARD
                .add(
                    None,
                    item,
                    quantity - self.has_available(item),
                    Purpose::Gear {
                        char: self.inner.name().to_owned(),
                        slot,
                        item_code: item.to_owned(),
                    },
                )
                .is_ok();
        }
        false
    }

    fn reserv_gear(&self, gear: &mut Gear) {
        gear.align_to(&self.gear());
        Slot::iter().for_each(|s| {
            if !(s.is_ring_1() || s.is_ring_2()) {
                if let Some(item) = gear.slot(s) {
                    let quantity = if s.is_utility_1() || s.is_utility_2() {
                        100
                    } else {
                        1
                    };
                    self.reserv_if_needed_and_available(s, &item.code, quantity);
                }
            }
        });
        if gear.ring1.is_some() && gear.ring1 == gear.ring2 {
            self.reserv_if_needed_and_available(Slot::Ring1, &gear.ring1.as_ref().unwrap().code, 2);
        } else {
            if let Some(ref ring1) = gear.ring1 {
                self.reserv_if_needed_and_available(Slot::Ring1, &ring1.code, 1);
            }
            if let Some(ref ring2) = gear.ring2 {
                self.reserv_if_needed_and_available(Slot::Ring2, &ring2.code, 1);
            }
        }
    }

    /// Reserves the given `quantity` of the `item` if needed and available.
    fn reserv_if_needed_and_available(&self, s: Slot, item: &str, quantity: i32) {
        if (self.equiped_in(s).is_none()
            || self
                .equiped_in(s)
                .is_some_and(|equiped| item != equiped.code))
            && self.inventory.total_of(item) < quantity
        {
            if let Err(e) = BANK.reserv(
                item,
                quantity - self.inventory.total_of(item),
                &self.inner.name(),
            ) {
                error!("{} failed to reserv '{}': {:?}", self.inner.name(), item, e)
            }
        }
    }

    pub fn unequip_and_deposit_all(&self) {
        Slot::iter().for_each(|s| {
            if let Some(item) = self.equiped_in(s) {
                let quantity = self.quantity_in_slot(s);
                if let Err(e) = self.unequip_item(s, quantity) {
                    error!(
                        "{}: failed to unequip '{}'x{} during unequip_and_deposit_all: {:?}",
                        self.inner.name(),
                        &item.code,
                        quantity,
                        e
                    )
                } else if let Err(e) = self.deposit_item(&item.code, quantity, None) {
                    error!(
                        "{}: failed to deposit '{}'x{} during `unequip_and_deposit_all`: {:?}",
                        self.inner.name(),
                        &item.code,
                        quantity,
                        e
                    )
                }
            }
        })
    }

    pub fn unequip_and_deposit_all_for_gathering(&self) {
        Slot::iter().for_each(|s| {
            if !s.is_weapon() {
                if let Some(item) = self.equiped_in(s) {
                    let quantity = self.quantity_in_slot(s);
                    if let Err(e) = self.unequip_item(s, quantity) {
                        error!(
                            "{}: failed to unequip '{}'x{} during unequip_and_deposit_all: {:?}",
                            self.inner.name(),
                            &item.code,
                            quantity,
                            e
                        )
                    } else if let Err(e) = self.deposit_item(&item.code, quantity, None) {
                        error!(
                            "{}: failed to deposit '{}'x{} during `unequip_and_deposit_all`: {:?}",
                            self.inner.name(),
                            &item.code,
                            quantity,
                            e
                        )
                    }
                }
            }
        })
    }

    pub fn skill_enabled(&self, s: Skill) -> bool {
        self.conf().read().unwrap().skills.contains(&s)
    }

    fn withdraw_food(&self) {
        let Ok(_browsed) = BANK.browsed.write() else {
            return;
        };
        if !self.inventory.consumable_food().is_empty() && !self.map().content_code_is("bank") {
            return;
        }
        let Some(food) = BANK
            .consumable_food(self.level())
            .into_iter()
            .filter(|f| BANK.has_available(&f.code, Some(&self.inner.name())) > 0)
            .max_by_key(|f| f.heal())
        else {
            return;
        };
        // TODO: defined quantity withdrowned depending on the monster drop rate and damages
        let quantity = min(
            self.inventory.max_items() - 30,
            BANK.has_available(&food.code, Some(&self.inner.name())),
        );
        if let Err(e) = BANK.reserv(&food.code, quantity, &self.inner.name()) {
            error!("{} failed to reserv food: {:?}", self.inner.name(), e)
        };
        drop(_browsed);
        // TODO: only deposit what is necessary, food already in inventory should be kept
        self.deposit_all();
        if let Err(e) = self.withdraw_item(&food.code, quantity) {
            error!("{} failed to withdraw food: {:?}", self.inner.name(), e)
        }
    }

    fn order_food(&self) {
        if !self.skill_enabled(Skill::Combat) {
            return;
        }
        self.inventory.consumable_food().iter().for_each(|f| {
            if let Err(e) = self
                .inventory
                .reserv(&f.code, self.inventory.total_of(&f.code))
            {
                error!(
                    "{} failed to reserv food currently in inventory: {:?}",
                    self.inner.name(),
                    e
                )
            }
        });
        if let Some(best_food) = ITEMS
            .best_consumable_foods(self.level())
            .iter()
            .max_by_key(|i| i.heal())
        {
            if BANK.has_available(&best_food.code, Some(&self.inner.name())) < MIN_FOOD_THRESHOLD {
                if let Err(e) = ORDER_BOARD.add_or_reset(
                    Some(&self.inner.name()),
                    &best_food.code,
                    ACCOUNT.fisher_max_items(),
                    Purpose::Food {
                        char: self.inner.name().to_owned(),
                    },
                ) {
                    error!(
                        "{} failed to add or reset food order: {:?}",
                        self.inner.name(),
                        e
                    )
                }
            }
        }
    }

    fn eat_food(&self) {
        self.inventory
            .consumable_food()
            .iter()
            .sorted_by_key(|i| i.heal())
            .for_each(|f| {
                // TODO: improve logic to eat different foods to restore more hp
                let mut quantity = self.missing_hp() / f.heal();
                if ACCOUNT.time_to_get(&f.code).is_some_and(|t| {
                    t * (self.missing_hp() / f.heal())
                        < FightSimulator::time_to_rest(self.missing_hp())
                }) {
                    quantity += 1;
                };
                if quantity > 0 {
                    if let Err(e) = self
                        .inner
                        .request_use_item(&f.code, min(quantity, self.inventory.total_of(&f.code)))
                    {
                        error!("{} failed to use food: {:?}", self.inner.name(), e)
                    }
                    self.inventory.decrease_reservation(&f.code, quantity);
                }
            });
    }
}

impl HasCharacterData for Character {
    fn data(&self) -> Arc<CharacterSchema> {
        self.inner.data().clone()
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

pub enum PostCraftAction {
    Deposit,
    Recycle,
    Keep,
}

#[derive(Error, Debug)]
pub enum CharacterError {
    #[error("Insuffisient skill level: {0} at level {1}")]
    InsuffisientSkillLevel(Skill, i32),
    #[error("Insuffisient materials")]
    InsuffisientMaterials,
    #[error("Invalid quantity")]
    InvalidQuantity,
    #[error("No gear to kill")]
    NoGearToKill,
    #[error("Map not found")]
    MapNotFound,
    #[error("Failed to move")]
    FailedToMove,
    #[error("Skill {0} is disabled")]
    SkillDisabled(Skill),
    #[error("Available gear is too weak to kill {monster_code}")]
    GearTooWeak { monster_code: String },
    #[error("Level insufficient")]
    LevelInsufficient,
    #[error("Item not craftable")]
    ItemNotCraftable,
    #[error("Item not found")]
    ItemNotFound,
    #[error("Character has no task")]
    NoTask,
    #[error("Character task is not finished")]
    TaskNotFinished,
    #[error("Not enough coin is available to the character")]
    NotEnoughCoin,
    #[error("Not enough gold is available to the character")]
    InsuffisientGold,
    #[error("Not enough gold is available in the bank")]
    InsuffisientGoldInBank,
    #[error("Not enough gold is available in the character inventory")]
    InsuffisientGoldInInventory,
    #[error("Bank is not available")]
    BankUnavailable,
    #[error("Inventory is full")]
    InventoryFull,
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Monster not found")]
    MonsterNotFound,
    #[error("Invalid task type")]
    InvalidTaskType,
    #[error("Missing item(s): '{item}'x{quantity}")]
    MissingItems { item: String, quantity: i32 },
    #[error("Quantity unavailable: {0}")]
    QuantityUnavailable(i32),
    #[error("Task already completed")]
    TaskAlreadyCompleted,
    #[error("Not enough gift is available to the character")]
    NotEnoughGift,
    #[error("Request error: {0}")]
    RequestError(RequestError),
    #[error("Insuffisient inventory space")]
    InsuffisientInventorySpace,
}

impl From<RequestError> for CharacterError {
    fn from(value: RequestError) -> Self {
        CharacterError::RequestError(value)
    }
}
