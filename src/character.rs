use crate::{
    account::AccountController,
    bank::Bank,
    bot_config::{BotConfig, CharConfig, Goal},
    error::{
        BankExpansionCommandError, BuyNpcCommandError, CraftCommandError, DeleteCommandError,
        DepositItemCommandError, EquipCommandError, GatherCommandError, GoldDepositCommandError,
        GoldWithdrawCommandError, KillMonsterCommandError, MoveCommandError, OrderDepositError,
        OrderProgresssionError, RecycleCommandError, SkillLevelingError,
        TaskAcceptationCommandError, TaskCancellationCommandError, TaskCompletionCommandError,
        TaskProgressionError, TaskTradeCommandError, TasksCoinExchangeCommandError,
        UnequipCommandError, UseItemCommandError, WithdrawItemCommandError,
    },
    gear_finder::{Filter, GearFinder},
    inventory::Inventory,
    leveling_helper::LevelingHelper,
    orderboard::{Order, OrderBoard, Purpose},
};
use anyhow::Result;
use artifactsmmo_sdk::{
    HasDrops, Items, Maps, Monsters, Server, Simulator,
    char::{Character as CharacterClient, HasCharacterData, Skill, error::RestError},
    consts::{
        BANK_MIN_FREE_SLOT, CRAFT_TIME, MAX_LEVEL, MIN_COIN_THRESHOLD, MIN_FOOD_THRESHOLD,
        TASK_CANCEL_PRICE, TASK_EXCHANGE_PRICE, TASKS_COIN,
    },
    gear::{Gear, Slot},
    items::{ItemSchemaExt, ItemSource},
    maps::MapSchemaExt,
    models::{
        CharacterSchema, FightResult, FightSchema, MapContentType, MapSchema, MonsterSchema,
        NpcSchema, RecyclingItemsSchema, ResourceSchema, RewardsSchema, SimpleItemSchema,
        SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, TaskType,
    },
    monsters::MonsterSchemaExt,
    npcs::Npcs,
    resources::ResourceSchemaExt,
};
use itertools::Itertools;
use log::{debug, error, info, warn};
use std::{
    cmp::min,
    option::Option,
    sync::{Arc, RwLock},
};
use strum::IntoEnumIterator;

#[derive(Default)]
pub struct CharacterController {
    config: Arc<BotConfig>,
    pub client: Arc<CharacterClient>,
    pub inventory: Arc<Inventory>,
    maps: Arc<Maps>,
    account: Arc<AccountController>,
    bank: Arc<Bank>,
    order_board: Arc<OrderBoard>,
    items: Arc<Items>,
    monsters: Arc<Monsters>,
    npcs: Arc<Npcs>,
    gear_finder: Arc<GearFinder>,
    leveling_helper: Arc<LevelingHelper>,
}

impl CharacterController {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Arc<BotConfig>,
        client: Arc<CharacterClient>,
        items: Arc<Items>,
        monsters: Arc<Monsters>,
        npcs: Arc<Npcs>,
        maps: Arc<Maps>,
        bank: Arc<Bank>,
        order_board: Arc<OrderBoard>,
        account: Arc<AccountController>,
        gear_finder: Arc<GearFinder>,
        leveling_helper: Arc<LevelingHelper>,
    ) -> Self {
        Self {
            config,
            inventory: Arc::new(Inventory::new(client.clone())),
            client,
            maps,
            items,
            monsters,
            npcs,
            bank,
            order_board,
            account,
            gear_finder,
            leveling_helper,
        }
    }

    pub fn run_loop(&self) {
        info!("{}: started !", self.name());
        loop {
            if self.conf().read().unwrap().idle {
                continue;
            }
            if self.inventory.is_full() {
                if let Err(e) = self.deposit_all() {
                    error!("Failed to deposit all in main loop: {}", e)
                }
                continue;
            }
            self.maps.refresh_from_events();
            self.order_food();
            if self.handle_goals() {
                continue;
            }
            // TODO: improve fallback
            match self.progress_task() {
                Ok(_) => continue,
                Err(TaskProgressionError::TaskTradeCommandError(
                    TaskTradeCommandError::MissingItems { item, quantity },
                )) => {
                    let _ = self.order_board.add(
                        &item,
                        quantity,
                        Some(&self.name()),
                        Purpose::Task {
                            char: self.name().to_owned(),
                        },
                    );
                    continue;
                }
                Err(_) => (),
            }
            for s in self.conf().read().unwrap().skills.iter() {
                if self.level_skill_up(*s).is_ok() {
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
                    self.level_skill_up(*skill).is_ok()
                }
                Goal::FollowMaxSkillLevel {
                    skill,
                    skill_to_follow,
                } if self.skill_level(*skill)
                    < min(
                        1 + self.account.max_skill_level(*skill_to_follow),
                        MAX_LEVEL,
                    ) =>
                {
                    self.level_skill_up(*skill).is_ok()
                }
                _ => false,
            })
    }

    fn level_skill_up(&self, skill: Skill) -> Result<(), SkillLevelingError> {
        if self.skill_level(skill) >= MAX_LEVEL {
            return Err(SkillLevelingError::SkillAlreadyMaxed);
        };
        if skill.is_combat() {
            return Ok(self.level_combat()?);
        }
        match self.level_skill_by_crafting(skill) {
            Ok(_) => Ok(()),
            Err(_) => Ok(self.level_skill_by_gathering(skill)?),
        }
    }

    fn level_skill_by_gathering(&self, skill: Skill) -> Result<(), GatherCommandError> {
        let Some(resource) = self
            .leveling_helper
            .best_resource(self.skill_level(skill), skill)
        else {
            return Err(GatherCommandError::MapNotFound);
        };
        self.gather_resource(&resource)?;
        Ok(())
    }

    fn level_skill_by_crafting(&self, skill: Skill) -> Result<(), CraftCommandError> {
        let Some(item) = self
            .leveling_helper
            .best_craft(self.skill_level(skill), skill, self)
        else {
            return Err(CraftCommandError::ItemNotFound);
        };
        let quantity = self.max_craftable_items(&item.code);
        match self.craft_from_bank(&item.code, quantity) {
            Ok(_) => {
                if !(skill.is_gathering() || skill.is_cooking())
                    && let Err(e) = self.recycle_item(&item.code, quantity)
                {
                    error!("Failed to recycle crafted items for leveling: {}", e)
                };
                Ok(())
            }
            Err(e) => {
                if let CraftCommandError::InsufficientMaterials = e
                    && (!skill.is_gathering()
                        || skill.is_alchemy()
                            && self
                                .leveling_helper
                                .best_resource(self.skill_level(skill), skill)
                                .is_none())
                    && self.order_missing_mats(
                        &item.code,
                        quantity,
                        Purpose::Leveling {
                            char: self.name().to_owned(),
                            skill,
                        },
                    )
                {
                    return Ok(());
                }
                Err(e)
            }
        }
    }

    /// Find a target and kill it if possible.
    fn level_combat(&self) -> Result<(), KillMonsterCommandError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(KillMonsterCommandError::SkillDisabled(Skill::Combat));
        }
        if let Ok(_) | Err(TaskCompletionCommandError::NoTask) = self.complete_task()
            && let Err(e) = self.accept_task(TaskType::Monsters)
        {
            error!("{} error while accepting new task: {:?}", self.name(), e)
        }
        if self.task_type().is_some_and(|t| t == TaskType::Monsters) && self.progress_task().is_ok()
        {
            return Ok(());
        }
        let Some(monster) = self.leveling_helper.best_monster(self) else {
            return Err(KillMonsterCommandError::MapNotFound);
        };
        self.kill_monster(&monster)?;
        Ok(())
    }

    /// Browse orderboard for completable orders: first check if some orders
    /// can be turned in, then check for completable orders (enough materials to craft all items
    /// from an order. Then check for orders that can be progressed. Then check for order for which
    /// the skill level required needs to be leveled.
    fn handle_orderboard(&self) -> bool {
        let orders = self.order_board.orders_by_priority();
        if orders.iter().cloned().any(|o| self.turn_in_order(o)) {
            return true;
        }
        let mut completable = orders
            .iter()
            .filter(|o| self.can_complete_order(o))
            .cloned();
        if completable.any(|r| self.handle_order(r).is_ok()) {
            return true;
        }
        let mut progressable = orders.into_iter().filter(|o| self.can_progress(o));
        if progressable.any(|r| self.handle_order(r).is_ok()) {
            return true;
        }
        false
    }

    /// Deposit items requiered by the given `order` if needed.
    /// Returns true if items has be deposited.
    fn turn_in_order(&self, order: Arc<Order>) -> bool {
        if self.order_board.should_be_turned_in(&order) {
            return self.deposit_order(&order).is_ok();
        }
        false
    }

    fn deposit_order(&self, order: &Order) -> Result<(), OrderDepositError> {
        let mut quantity = self.inventory.has_available(&order.item);
        if quantity <= 0 {
            return Err(OrderDepositError::NoItemToDeposit);
        }
        quantity = min(quantity, order.missing());
        self.deposit_item(&order.item, quantity, order.owner.clone())?;
        Ok(())
    }

    fn handle_order(&self, order: Arc<Order>) -> Result<i32, OrderProgresssionError> {
        match self.progress_order(&order) {
            Ok(progress) => {
                if progress > 0 {
                    info!(
                        "{}: progressed by {} on order: {}, in inventories: {}",
                        self.name(),
                        progress,
                        order,
                        self.account.available_in_inventories(&order.item),
                    );
                }
                self.turn_in_order(order);
                Ok(progress)
            }
            Err(err) => {
                debug!(
                    "{}: no progress done on order {}: {}",
                    self.name(),
                    order,
                    err
                );
                Err(err)
            }
        }
    }

    fn can_progress(&self, order: &Order) -> bool {
        self.items
            .best_source_of(&order.item)
            .iter()
            .any(|s| match s {
                ItemSource::Resource(r) => self.can_gather(r).is_ok(),
                ItemSource::Monster(m) => self.can_kill(m).is_ok(),
                ItemSource::Craft => self.can_craft(&order.item).is_ok(),
                ItemSource::TaskReward => order.in_progress() <= 0,
                ItemSource::Task => true,
                ItemSource::Npc(_) => true,
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
        self.items
            .mats_for(item, quantity)
            .into_iter()
            .filter(|m| self.bank.has_available(&m.code, Some(&self.name())) < m.quantity)
            .update(|m| {
                m.quantity -= if self.order_board.is_ordered(&m.code) {
                    0
                } else {
                    self.bank.has_available(&m.code, Some(&self.name()))
                }
            })
            .for_each(|m| {
                if self
                    .order_board
                    .add(&m.code, m.quantity, None, purpose.clone())
                    .is_ok()
                {
                    ordered = true
                }
            });
        ordered
    }

    /// Checks if the character is able to get the missing items for the `order` in one command
    /// Resource and Monsters sources return false because drop rate might not be 100%
    /// TODO: maybe check drop rate of item and return `true` if it is 100%
    fn can_complete_order(&self, order: &Order) -> bool {
        self.items
            .best_source_of(&order.item)
            .iter()
            .any(|s| match s {
                ItemSource::Resource(_) => false,
                ItemSource::Monster(_) => false,
                ItemSource::Craft => {
                    self.can_craft(&order.item).is_ok()
                        && self
                            .bank
                            .missing_mats_for(
                                &order.item,
                                self.order_board.total_missing_for(order),
                                Some(&self.name()),
                            )
                            .is_empty()
                }
                ItemSource::TaskReward => {
                    self.has_available(TASKS_COIN) >= TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
                }
                ItemSource::Task => self.has_available(&self.task()) >= self.task_missing(),
                ItemSource::Npc(n) => self
                    .npcs
                    .items
                    .get(&order.item)
                    .map(|i| (i.currency.clone(), i.buy_price))
                    .is_some_and(|(c, p)| {
                        self.has_available(&c) > p.unwrap_or(0) * order.quantity()
                    }),
            })
    }

    fn progress_order(&self, order: &Order) -> Result<i32, OrderProgresssionError> {
        if self.order_board.total_missing_for(order) <= 0 {
            return Err(OrderProgresssionError::NoItemMissing);
        }
        let Some(source) = self.items.best_source_of(&order.item) else {
            return Err(OrderProgresssionError::NoSourceForItem);
        };
        Ok(match source {
            ItemSource::Resource(r) => self.progress_resource_order(order, &r)?,
            ItemSource::Monster(m) => self.progress_monster_order(order, &m)?,
            ItemSource::Craft => self.progress_crafting_order(order)?,
            ItemSource::TaskReward => self.progress_task_reward_order(order)?,
            ItemSource::Task => self.progress_task_order(order)?,
            ItemSource::Npc(n) => self.progress_npc_order(order, &n)?,
        })
    }

    fn progress_resource_order(
        &self,
        order: &Order,
        r: &ResourceSchema,
    ) -> Result<i32, GatherCommandError> {
        order.inc_in_progress(1);
        let result = self
            .gather_resource(r)
            .map(|gather| gather.amount_of(&order.item));
        order.dec_in_progress(1);
        result
    }

    fn progress_monster_order(
        &self,
        order: &Order,
        m: &MonsterSchema,
    ) -> Result<i32, KillMonsterCommandError> {
        self.kill_monster(m)
            .map(|fight| fight.amount_of(&order.item))
    }

    fn progress_crafting_order(&self, order: &Order) -> Result<i32, CraftCommandError> {
        self.can_craft(&order.item)?;
        if self.order_missing_mats(
            &order.item,
            self.order_board.total_missing_for(order),
            order.purpose.clone(),
        ) {
            return Ok(0);
        }
        let quantity = min(
            self.max_craftable_items(&order.item),
            self.order_board.total_missing_for(order),
        );
        if quantity <= 0 {
            return Err(CraftCommandError::InsufficientMaterials);
        }
        order.inc_in_progress(quantity);
        let crafted = self.craft_from_bank(&order.item, quantity);
        order.dec_in_progress(quantity);
        crafted.map(|craft| craft.amount_of(&order.item))
    }

    fn progress_task_reward_order(
        &self,
        order: &Order,
    ) -> Result<i32, TasksCoinExchangeCommandError> {
        match self.can_exchange_task() {
            Ok(()) => {
                order.inc_in_progress(1);
                let exchanged = self.exchange_task().map(|r| r.amount_of(&order.item));
                order.dec_in_progress(1);
                exchanged
            }
            Err(e) => {
                if self.order_board.total_missing_for(order) <= 0 {
                    return Err(e);
                }
                if let TasksCoinExchangeCommandError::NotEnoughCoins = e {
                    let q = TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
                        - if self.order_board.is_ordered(TASKS_COIN) {
                            0
                        } else {
                            self.has_in_bank_or_inv(TASKS_COIN)
                        };
                    return self
                        .order_board
                        .add(TASKS_COIN, q, None, order.purpose.to_owned())
                        .map(|_| 0)
                        //TODO: improve error handling, this variant should not exist
                        .map_err(|_| TasksCoinExchangeCommandError::OrderError);
                }
                Err(e)
            }
        }
    }

    fn progress_task_order(&self, order: &Order) -> Result<i32, TaskProgressionError> {
        match self.complete_task() {
            Ok(r) => Ok(r.amount_of(&order.item)),
            Err(e) => {
                if let TaskCompletionCommandError::NoTask = e {
                    let r#type = self.conf().read().unwrap().task_type;
                    if let Err(e) = self.accept_task(r#type) {
                        error!("{} error while accepting new task: {:?}", self.name(), e)
                    }
                    return Ok(0);
                }
                let TaskCompletionCommandError::TaskNotFinished = e else {
                    return Err(e.into());
                };
                match self.progress_task() {
                    Ok(_) => Ok(0),
                    Err(TaskProgressionError::TaskTradeCommandError(
                        TaskTradeCommandError::MissingItems { item, quantity },
                    )) => self
                        .order_board
                        .add(&item, quantity, Some(&self.name()), order.purpose.clone())
                        .map(|_| 0)
                        //TODO: better error handling, variant should not exist ?
                        .map_err(|_| TaskProgressionError::OrderError),
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn progress_npc_order(
        &self,
        order: &Order,
        npc: &NpcSchema,
    ) -> Result<i32, BuyNpcCommandError> {
        todo!()
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
    //            if self.order_board.total_missing_for(order) <= 0 {
    //                return None;
    //            }
    //            if let CharacterError::NotEnoughGift = e {
    //                let q = 1 - if self.order_board.is_ordered(GIFT) {
    //                    0
    //                } else {
    //                    self.has_in_bank_or_inv(GIFT)
    //                };
    //                return self.order_board
    //                    .add(None, GIFT, q, order.purpose.to_owned())
    //                    .ok()
    //                    .map(|_| 0);
    //            }
    //            None
    //        }
    //    }
    //}

    fn progress_task(&self) -> Result<(), TaskProgressionError> {
        if self.task().is_empty() {
            let r#type = self.conf().read().unwrap().task_type;
            return Ok(self.accept_task(r#type).map(|_| ())?);
        }
        if self.task_finished() {
            return Ok(self.complete_task().map(|_| ())?);
        }
        let Some(monster) = self.monsters.get(&self.task()) else {
            return Ok(self.trade_task().map(|_| ())?);
        };
        match self.kill_monster(&monster) {
            Ok(_) => Ok(()),
            Err(e) => {
                if let KillMonsterCommandError::GearTooWeak { monster_code: _ } = e {
                    warn!("{}: {}", self.name(), e);
                    self.cancel_task()?;
                    Ok(())
                } else {
                    Err(e.into())
                }
            }
        }
    }

    fn trade_task(&self) -> Result<TaskTradeSchema, TaskTradeCommandError> {
        self.can_trade_task()?;
        let q = min(self.task_missing(), self.inventory.max_items());
        if let Err(e) = self.bank.reserv(&self.task(), q, &self.name()) {
            error!(
                "{}: error while reserving items for item task: {:?}",
                self.name(),
                e
            )
        }
        if let Err(e) = self.deposit_all() {
            error!("Failed to deposit all while task trading: {}", e)
        }
        if let Err(e) = self.withdraw_item(&self.task(), q) {
            error!("{}: error while withdrawing {:?}", self.name(), e);
            self.bank
                .decrease_reservation(&self.task(), q, &self.name());
        };
        self.move_to_closest_taskmaster(self.task_type())?;
        let res = self.client.task_trade(&self.task(), q);
        self.inventory.decrease_reservation(&self.task(), q);
        Ok(res?)
    }

    fn can_trade_task(&self) -> Result<(), TaskTradeCommandError> {
        if self.task().is_empty() {
            return Err(TaskTradeCommandError::NoTask);
        }
        if self.task_type().is_none_or(|tt| tt != TaskType::Items) {
            return Err(TaskTradeCommandError::InvalidTaskType);
        }
        if self.task_missing() <= 0 {
            return Err(TaskTradeCommandError::TaskAlreadyCompleted);
        }
        if self.task_missing()
            > self.bank.has_available(&self.task(), Some(&self.name()))
                + self.inventory.total_of(&self.task())
        {
            return Err(TaskTradeCommandError::MissingItems {
                item: self.task().to_owned(),
                quantity: self.task_missing()
                    - self.bank.has_available(&self.task(), Some(&self.name()))
                    - self.inventory.total_of(&self.task()),
            });
        }
        Ok(())
    }

    fn accept_task(&self, r#type: TaskType) -> Result<TaskSchema, TaskAcceptationCommandError> {
        self.move_to_closest_taskmaster(Some(r#type))?;
        Ok(self.client.accept_task()?)
    }

    fn complete_task(&self) -> Result<RewardsSchema, TaskCompletionCommandError> {
        if self.task().is_empty() {
            return Err(TaskCompletionCommandError::NoTask);
        }
        if !self.task_finished() {
            return Err(TaskCompletionCommandError::TaskNotFinished);
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        Ok(self.client.complete_task()?)
    }

    fn can_exchange_task(&self) -> Result<(), TasksCoinExchangeCommandError> {
        if self.has_in_bank_or_inv(TASKS_COIN) < TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD {
            return Err(TasksCoinExchangeCommandError::NotEnoughCoins);
        }
        Ok(())
    }

    fn exchange_task(&self) -> Result<RewardsSchema, TasksCoinExchangeCommandError> {
        self.can_exchange_task()?;
        let mut quantity = min(
            self.inventory.max_items() / 2,
            self.bank.has_available(TASKS_COIN, Some(&self.name())),
        );
        quantity = quantity - (quantity % TASK_EXCHANGE_PRICE);
        if self.inventory.total_of(TASKS_COIN) >= TASK_EXCHANGE_PRICE {
            if let Err(e) = self
                .inventory
                .reserv(TASKS_COIN, self.inventory.total_of(TASKS_COIN))
            {
                error!(
                    "{}: error while reserving tasks coins in inventory: {}",
                    self.name(),
                    e
                );
            }
        } else {
            if self
                .bank
                .reserv(TASKS_COIN, quantity, &self.name())
                .is_err()
            {
                return Err(TasksCoinExchangeCommandError::NotEnoughCoins);
            }
            if let Err(e) = self.deposit_all_but(TASKS_COIN) {
                error!("Failed to deposit all while exchanging task: {}", e)
            }
            self.withdraw_item(TASKS_COIN, quantity)?;
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        let result = self.client.exchange_tasks_coin().map_err(|e| e.into());
        self.inventory
            .decrease_reservation(TASKS_COIN, TASK_EXCHANGE_PRICE);
        result
    }

    //fn can_exchange_gift(&self) -> Result<(), CharacterError> {
    //    if self.inventory.total_of(GIFT) + self.bank.has_available(GIFT, Some(&self.inner.name())) < 1 {
    //        return Err(CharacterError::NotEnoughGift);
    //    }
    //    Ok(())
    //}

    //fn exchange_gift(&self) -> Result<RewardsSchema, CharacterError> {
    //    self.can_exchange_gift()?;
    //    let quantity = min(
    //        self.inventory.max_items() / 2,
    //        self.bank.has_available(GIFT, Some(&self.inner.name())),
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
    //        if self.bank.reserv(GIFT, quantity, &self.inner.name()).is_err() {
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

    fn cancel_task(&self) -> Result<(), TaskCancellationCommandError> {
        if self.bank.has_available(TASKS_COIN, Some(&self.name()))
            < TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD
        {
            return Err(TaskCancellationCommandError::NotEnoughCoins);
        }
        if self.inventory.has_available(TASKS_COIN) <= 0 {
            if self
                .bank
                .reserv("tasks_coin", TASK_CANCEL_PRICE, &self.name())
                .is_err()
            {
                return Err(TaskCancellationCommandError::NotEnoughCoins);
            }
            if let Err(e) = self.deposit_all() {
                error!("Failed to deposit all while canceling task: {}", e)
            }
            self.withdraw_item(TASKS_COIN, TASK_CANCEL_PRICE)?;
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        let result = self.client.cancel_task().map_err(|e| e.into());
        self.inventory
            .decrease_reservation(TASKS_COIN, TASK_CANCEL_PRICE);
        result
    }

    /// Checks if an gear making the `Character` able to kill the given
    /// `monster` is available, equip it, then move the `Character` to the given
    /// map or the closest containing the `monster` and fight it.
    fn kill_monster(
        &self,
        monster: &MonsterSchema,
    ) -> Result<FightSchema, KillMonsterCommandError> {
        self.can_fight(monster)?;
        if let Ok(_) | Err(TaskCompletionCommandError::NoTask) = self.complete_task()
            && let Err(e) = self.accept_task(TaskType::Monsters)
        {
            error!("{} error while accepting new task: {:?}", self.name(), e)
        }
        if self.inventory.free_space() < monster.max_drop_quantity()
            || self
                .client
                .current_map()
                .content_type_is(MapContentType::Bank)
        {
            self.deposit_all()?;
        };
        self.check_for_combat_gear(monster)?;
        self.withdraw_food();
        if !self.can_kill_now(monster) {
            self.eat_food_from_inventory();
        }
        if !self.can_kill_now(monster)
            && let Err(e) = self.rest()
        {
            error!("{} failed to rest: {:?}", self.name(), e)
        }
        self.move_to_closest_map_with_content_code(&monster.code)?;
        Ok(self.client.fight()?)
    }

    fn check_for_combat_gear(
        &self,
        monster: &MonsterSchema,
    ) -> Result<(), KillMonsterCommandError> {
        let mut available: Gear;
        let Ok(_browsed) = self.bank.browsed.write() else {
            return Err(KillMonsterCommandError::BankUnavailable);
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

    fn order_best_gear_against(&self, monster: &MonsterSchema) {
        let mut gear = self.gear_finder.best_winning_against(
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

    fn rest(&self) -> Result<i32, RestError> {
        if self.health() < self.max_health() {
            Ok(self.client.rest()?)
        } else {
            Ok(0)
        }
    }

    /// Checks if the character is able to gather the given `resource`. If it
    /// can, equips the best available appropriate tool, then move the `Character`
    /// to the given map or the closest containing the `resource` and gather it.  
    fn gather_resource(
        &self,
        resource: &ResourceSchema,
    ) -> Result<SkillDataSchema, GatherCommandError> {
        self.can_gather(resource)?;
        let Some(map) = self
            .maps
            .closest_with_content_code_from(self.client.current_map(), &resource.code)
        else {
            return Err(GatherCommandError::MapNotFound);
        };
        if self.inventory.free_space() < resource.max_drop_quantity()
            || self
                .client
                .current_map()
                .content_type_is(MapContentType::Bank)
        {
            self.deposit_all()?;
        };
        self.check_for_skill_gear(resource.skill.into());
        self.r#move(map.x, map.y)?;
        Ok(self.client.gather()?)
    }

    // Checks that the `Character` has the required skill level to gather the given `resource`
    fn can_gather(&self, resource: &ResourceSchema) -> Result<(), GatherCommandError> {
        let skill: Skill = resource.skill.into();
        if !self.skill_enabled(skill) {
            return Err(GatherCommandError::SkillDisabled(skill));
        }
        if self.client.skill_level(skill) < resource.level {
            return Err(GatherCommandError::InsufficientSkillLevel(skill));
        }
        Ok(())
    }

    fn check_for_skill_gear(&self, skill: Skill) {
        let Ok(_browsed) = self.bank.browsed.write() else {
            return;
        };
        let mut available = self.gear_finder.best_for_skill(
            self,
            skill,
            Filter {
                available: true,
                ..Default::default()
            },
        );
        self.reserv_gear(&mut available);
        self.order_best_gear_for_skill(skill);
        drop(_browsed);
        self.equip_gear(&mut available);
    }

    fn order_best_gear_for_skill(&self, skill: Skill) {
        let mut gear = self.gear_finder.best_for_skill(
            self,
            skill,
            Filter {
                can_craft: true,
                from_task: false,
                from_monster: false,
                ..Default::default()
            },
        );
        self.order_gear(&mut gear);
    }

    pub fn can_fight(&self, monster: &MonsterSchema) -> Result<(), KillMonsterCommandError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(KillMonsterCommandError::SkillDisabled(Skill::Combat));
        }
        if self.maps.with_content_code(&monster.code).is_empty() {
            return Err(KillMonsterCommandError::MapNotFound);
        }
        Ok(())
    }

    /// Checks if the `Character` is able to kill the given monster and returns
    /// the best available gear to do so.
    pub fn can_kill<'a>(
        &'a self,
        monster: &'a MonsterSchema,
    ) -> Result<Gear, KillMonsterCommandError> {
        self.can_fight(monster)?;
        let available = self.gear_finder.best_winning_against(
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
            Err(KillMonsterCommandError::GearTooWeak {
                monster_code: monster.code.to_owned(),
            })
        }
    }

    /// Checks if the `Character` could kill the given `monster` with the given
    /// `gear`
    fn can_kill_with(&self, monster: &MonsterSchema, gear: &Gear) -> bool {
        Simulator::fight(self.client.level(), 0, gear, monster, false).result == FightResult::Win
    }

    fn can_kill_now(&self, monster: &MonsterSchema) -> bool {
        Simulator::fight(
            self.level(),
            self.missing_hp(),
            &self.client.gear(),
            monster,
            false,
        )
        .result
            == FightResult::Win
    }

    /// Crafts the given `quantity` of the given item `code` if the required
    /// materials to craft them in one go are available in bank and deposit the crafted
    /// items into the bank.
    pub fn craft_from_bank(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SkillInfoSchema, CraftCommandError> {
        self.can_craft(item)?;
        let Some(item) = self.items.get(item) else {
            return Err(CraftCommandError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CraftCommandError::ItemNotCraftable);
        };
        if self.max_craftable_items_from_bank(&item.code) < quantity {
            return Err(CraftCommandError::InsufficientMaterials);
        }
        info!(
            "{}: going to craft '{}'x{} from bank.",
            self.name(),
            item.code,
            quantity
        );
        let mats = self.items.mats_for(&item.code, quantity);
        mats.iter().for_each(|m| {
            if let Err(e) = self.bank.reserv(&m.code, m.quantity, &self.name()) {
                error!(
                    "{}: error while reserving mats for crafting from bank: {:?}",
                    self.name(),
                    e
                )
            }
        });
        self.check_for_skill_gear(skill);
        self.deposit_all()?;
        self.withdraw_items(&mats)?;
        let Some(map) = self.maps.with_workshop_for(skill) else {
            return Err(MoveCommandError::MapNotFound.into());
        };
        self.r#move(map.x, map.y)?;
        let craft = self.client.craft(&item.code, quantity)?;
        mats.iter().for_each(|m| {
            self.inventory.decrease_reservation(&m.code, m.quantity);
        });
        Ok(craft)
    }

    // Checks that the `Character` has the required skill level to craft the given item `code`
    pub fn can_craft(&self, item: &str) -> Result<(), CraftCommandError> {
        let Some(item) = self.items.get(item) else {
            return Err(CraftCommandError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CraftCommandError::ItemNotCraftable);
        };
        if !self.skill_enabled(skill) {
            return Err(CraftCommandError::SkillDisabled(skill));
        }
        if self.client.skill_level(skill) < item.level {
            return Err(CraftCommandError::InsufficientSkillLevel(skill, item.level));
        }
        // TODO: improve condition
        if self.inventory.is_full() {
            return Err(CraftCommandError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn recycle_item(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<RecyclingItemsSchema, RecycleCommandError> {
        self.can_recycle(item, quantity)?;
        let Some(item) = self.items.get(item) else {
            return Err(RecycleCommandError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(RecycleCommandError::ItemNotCraftable);
        };
        let quantity_available = self.has_in_bank_or_inv(&item.code);
        if quantity_available < quantity {
            return Err(RecycleCommandError::InsufficientQuantity);
        }
        info!(
            "{}: going to recycle '{}x{}'.",
            self.name(),
            &item.code,
            quantity
        );
        if self.inventory.total_of(&item.code) < quantity {
            let missing_quantity = quantity - self.inventory.has_available(&item.code);
            if let Err(e) = self.bank.reserv(&item.code, missing_quantity, &self.name()) {
                error!(
                    "{}: error while reserving '{}': {:?}",
                    self.name(),
                    &item.code,
                    e
                );
            }
            if let Err(e) = self.deposit_all_but(&item.code) {
                error!("Failed to deposit all while recycling from bank: {}", e)
            }
            self.withdraw_item(&item.code, missing_quantity)?;
        }
        let Some(map) = self.maps.with_workshop_for(skill) else {
            return Err(MoveCommandError::MapNotFound.into());
        };
        self.r#move(map.x, map.y)?;
        let result = self.client.recycle(&item.code, quantity);
        self.inventory.decrease_reservation(&self.task(), quantity);
        Ok(result?)
    }

    pub fn can_recycle(&self, item: &str, quantity: i32) -> Result<(), RecycleCommandError> {
        let Some(item) = self.items.get(item) else {
            return Err(RecycleCommandError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(RecycleCommandError::ItemNotCraftable);
        };
        if !self.skill_enabled(skill) {
            return Err(RecycleCommandError::SkillDisabled(skill));
        };
        if self.client.skill_level(skill) < item.level {
            return Err(RecycleCommandError::InsufficientSkillLevel(
                skill, item.level,
            ));
        };
        if self.inventory.max_items() < item.recycled_quantity() * quantity {
            return Err(RecycleCommandError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn delete_item(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, DeleteCommandError> {
        if self.has_in_bank_or_inv(item) < quantity {
            return Err(DeleteCommandError::InsufficientQuantity);
        }
        info!("{}: going to delete '{}x{}'.", self.name(), item, quantity);
        if self.inventory.has_available(item) < quantity {
            let missing_quantity = quantity - self.inventory.has_available(item);
            if let Err(e) = self.bank.reserv(item, missing_quantity, &self.name()) {
                error!("{}: error while reserving '{}': {:?}", self.name(), item, e);
            }
            if let Err(e) = self.deposit_all_but(item) {
                error!(
                    "Failed to deposit all but {} while deleting item: {}",
                    item, e
                )
            }
            self.withdraw_item(item, missing_quantity)?;
        }
        let result = self.client.delete(item, quantity);
        self.inventory.decrease_reservation(&self.task(), quantity);
        Ok(result?)
    }

    /// Deposits all the gold and items in the character inventory into the bank.
    /// Items needed by orders are turned in first.
    pub fn deposit_all(&self) -> Result<(), DepositItemCommandError> {
        if self.inventory.total_items() <= 0 {
            return Ok(());
        }
        info!("{}: going to deposit all items to the bank.", self.name(),);
        self.deposit_items(&self.inventory.simple_content(), None)
    }

    pub fn deposit_all_but(&self, item: &str) -> Result<(), DepositItemCommandError> {
        if self.inventory.total_items() <= 0 {
            return Ok(());
        }
        info!(
            "{}: going to deposit all items but '{item}' to the bank.",
            self.name(),
        );
        let mut items = self.inventory.simple_content();
        items.retain(|i| i.code != item);
        self.deposit_items(&items, None)
    }

    pub fn deposit_item(
        &self,
        item: &str,
        quantity: i32,
        owner: Option<String>,
    ) -> Result<(), DepositItemCommandError> {
        self.deposit_items(
            &[SimpleItemSchema {
                code: item.to_string(),
                quantity,
            }],
            owner,
        )
    }

    /// TODO: finish implementing, a check for bank space and expansion
    pub fn deposit_items(
        &self,
        items: &[SimpleItemSchema],
        owner: Option<String>,
    ) -> Result<(), DepositItemCommandError> {
        if items.is_empty() {
            return Ok(());
        }
        if items
            .iter()
            .any(|i| self.inventory.total_of(&i.code) < i.quantity)
        {
            return Err(DepositItemCommandError::MissingQuantity);
        }
        let items_not_in_bank = items
            .iter()
            .filter(|i| self.bank.total_of(&i.code) <= 0)
            .count() as i32;
        if self.bank.details().slots < items_not_in_bank {
            return Err(DepositItemCommandError::InsufficientBankSpace);
        };
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        if self.bank.free_slots() <= BANK_MIN_FREE_SLOT
            && let Err(e) = self.expand_bank()
        {
            error!("{}: failed to expand bank capacity: {:?}", self.name(), e)
        }
        let deposit = self.client.deposit_item(items);
        match deposit {
            Ok(_) => {
                self.order_board.register_deposited_items(items, &owner);
                if let Some(ref owner) = owner {
                    items.iter().for_each(|i| {
                        if let Err(e) = self.bank.increase_reservation(&i.code, i.quantity, owner) {
                            error!("{}: failed to reserv deposited item: {:?}", self.name(), e)
                        }
                    })
                }
                items.iter().for_each(|i| {
                    self.inventory.decrease_reservation(&i.code, i.quantity);
                });
            }
            Err(ref e) => error!(
                "{}: error while depositing items ({:?}): {}",
                self.name(),
                items,
                e
            ),
        }
        if let Err(e) = self.deposit_all_gold() {
            error!(
                "{}: failed to deposit gold to the bank: {:?}",
                self.name(),
                e
            )
        }
        Ok(deposit?)
    }

    fn withdraw_food(&self) {
        let Ok(_browsed) = self.bank.browsed.write() else {
            return;
        };
        if !self.inventory.consumable_food().is_empty()
            && !self.client.current_map().content_code_is("bank")
        {
            return;
        }
        let Some(food) = self
            .bank
            .consumable_food(self.level())
            .into_iter()
            .filter(|f| self.bank.has_available(&f.code, Some(&self.name())) > 0)
            .max_by_key(|f| f.heal())
        else {
            return;
        };
        // TODO: defined quantity withdrowned depending on the monster drop rate and damages
        let quantity = min(
            self.inventory.max_items() - 30,
            self.bank.has_available(&food.code, Some(&self.name())),
        );
        if let Err(e) = self.bank.reserv(&food.code, quantity, &self.name()) {
            error!("{} failed to reserv food: {:?}", self.name(), e)
        };
        drop(_browsed);
        // TODO: only deposit what is necessary, food already in inventory should be kept
        if let Err(e) = self.deposit_all() {
            error!("Failed to deposit all while withdrawing food: {}", e)
        }
        if let Err(e) = self.withdraw_item(&food.code, quantity) {
            error!("{} failed to withdraw food: {:?}", self.name(), e)
        }
    }

    pub fn withdraw_item(&self, item: &str, quantity: i32) -> Result<(), WithdrawItemCommandError> {
        self.withdraw_items(&[SimpleItemSchema {
            code: item.to_string(),
            quantity,
        }])
    }

    /// Withdraw items from bank.
    /// Does not `deposit_all` before withdrawing because the caller might want to keep
    /// items reserved
    // TODO: maybe add optionnal parameter to deposit_all
    ///TODO: maybe reserve item before withdrawing
    pub fn withdraw_items(
        &self,
        items: &[SimpleItemSchema],
    ) -> Result<(), WithdrawItemCommandError> {
        if items.is_empty() {
            return Ok(());
        }
        if items
            .iter()
            .any(|i| self.bank.has_available(&i.code, Some(&self.name())) < i.quantity)
        {
            return Err(WithdrawItemCommandError::InsufficientQuantity);
        }
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        let result = self.client.withdraw_item(items);
        if result.is_ok() {
            items.iter().for_each(|i| {
                self.bank
                    .decrease_reservation(&i.code, i.quantity, &self.name());
                if let Err(e) = self.inventory.reserv(&i.code, i.quantity) {
                    error!(
                        "{}: failed to reserv withdrawed item '{}'x{}: {:?}",
                        self.name(),
                        i.code,
                        i.quantity,
                        e
                    );
                }
            });
        }
        Ok(result?)
    }

    pub fn deposit_all_gold(&self) -> Result<i32, GoldDepositCommandError> {
        self.deposit_gold(self.gold())
    }

    pub fn deposit_gold(&self, amount: i32) -> Result<i32, GoldDepositCommandError> {
        if amount <= 0 {
            return Ok(0);
        };
        if amount > self.gold() {
            return Err(GoldDepositCommandError::InsufficientGold);
        }
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        Ok(self.client.deposit_gold(amount)?)
    }

    pub fn withdraw_gold(&self, amount: i32) -> Result<i32, GoldWithdrawCommandError> {
        if amount <= 0 {
            return Ok(0);
        };
        if self.bank.gold() < amount {
            return Err(GoldWithdrawCommandError::InsufficientGold);
        };
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        Ok(self.client.withdraw_gold(amount)?)
    }

    pub fn expand_bank(&self) -> Result<i32, BankExpansionCommandError> {
        let Ok(_being_expanded) = self.bank.being_expanded.try_write() else {
            return Err(BankExpansionCommandError::BankUnavailable);
        };
        if self.bank.gold() + self.gold() < self.bank.next_expansion_cost() {
            return Err(BankExpansionCommandError::InsufficientGold);
        };
        let missing_gold = self.bank.next_expansion_cost() - self.gold();
        self.withdraw_gold(missing_gold)?;
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        Ok(self.client.expand_bank()?)
    }

    pub fn empty_bank(&self) -> Vec<Result<()>> {
        self.bank
            .content()
            .iter()
            .map(|i| -> Result<()> {
                info!("{}: deleting all '{}' from bank.", self.name(), i.code);
                let mut remain = i.quantity;
                while remain > 0 {
                    self.deposit_all()?;
                    let quantity = min(self.inventory.free_space(), remain);
                    self.delete_item(&i.code, quantity)?;
                    remain -= quantity;
                }
                Ok(())
            })
            .collect_vec()
    }

    fn equip_gear(&self, gear: &mut Gear) {
        gear.align_to(&self.client.gear());
        Slot::iter().for_each(|s| {
            if let Some(item) = gear.slot(s) {
                self.equip_from_inventory_or_bank(&item.code, s);
            }
        });
    }

    fn equip_from_inventory_or_bank(&self, item: &str, slot: Slot) {
        let prev_equiped = self.items.get(&self.equiped_in(slot));
        if prev_equiped.as_ref().is_some_and(|e| e.code == item) {
            return;
        }
        if self.inventory.total_of(item) <= 0
            && self.bank.has_available(item, Some(&self.name())) > 0
        {
            let quantity = min(
                slot.max_quantity(),
                self.bank.has_available(item, Some(&self.name())),
            );
            if self.inventory.free_space() < quantity
                && let Err(e) = self.deposit_all()
            {
                error!(
                    "Failed to deposit all while equiping item from bank or inventory: {}",
                    e
                )
            }
            if let Err(e) = self.withdraw_item(item, quantity) {
                error!(
                    "{} failed withdraw item from bank or inventory: {:?}",
                    self.name(),
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
                self.name(),
                e
            );
        }
        if let Some(i) = prev_equiped
            && self.inventory.total_of(&i.code) > 0
            && let Err(e) = self.deposit_item(&i.code, self.inventory.total_of(&i.code), None)
        {
            error!(
                "{} failed to deposit previously equiped item: {:?}",
                self.name(),
                e
            );
        }
    }

    fn equip_item(
        &self,
        item_code: &str,
        slot: Slot,
        quantity: i32,
    ) -> Result<(), EquipCommandError> {
        let Some(item) = self.items.get(item_code) else {
            return Err(EquipCommandError::ItemNotFound);
        };
        if self.inventory.free_space() + item.inventory_space() <= 0
            && let Err(e) = self.deposit_all_but(item_code)
        {
            error!(
                "Failed to deposit all but {} while equiping item: {}",
                item_code, e
            )
        }
        self.unequip_slot(slot, self.quantity_in_slot(slot))?;
        self.client.equip(item_code, slot, quantity)?;
        self.inventory.decrease_reservation(item_code, quantity);
        Ok(())
    }

    pub fn unequip_and_deposit_all(&self) {
        Slot::iter().for_each(|s| {
            if let Some(item) = self.items.get(&self.equiped_in(s)) {
                let quantity = self.quantity_in_slot(s);
                if let Err(e) = self.unequip_slot(s, quantity) {
                    error!(
                        "{}: failed to unequip '{}'x{} during unequip_and_deposit_all: {:?}",
                        self.name(),
                        &item.code,
                        quantity,
                        e
                    )
                } else if let Err(e) = self.deposit_item(&item.code, quantity, None) {
                    error!(
                        "{}: failed to deposit '{}'x{} during `unequip_and_deposit_all`: {:?}",
                        self.name(),
                        &item.code,
                        quantity,
                        e
                    )
                }
            }
        })
    }

    fn unequip_slot(&self, slot: Slot, quantity: i32) -> Result<(), UnequipCommandError> {
        let Some(equiped) = self.items.get(&self.equiped_in(slot)) else {
            return Ok(());
        };
        if !self.inventory.has_space_for(&equiped.code, quantity) {
            return Err(UnequipCommandError::InsufficientInventorySpace);
        }
        if self.client.health() <= equiped.health() {
            self.eat_food_from_inventory();
        }
        if self.client.health() <= equiped.health() {
            self.rest()?;
        }
        Ok(self.client.unequip(slot, quantity)?)
    }

    fn move_to_closest_taskmaster(
        &self,
        task_type: Option<TaskType>,
    ) -> Result<Arc<MapSchema>, MoveCommandError> {
        let current_map = self.current_map();
        if self.current_map().is_tasksmaster(task_type) {
            return Ok(current_map);
        }
        let Some(map) = self
            .maps
            .closest_tasksmaster_from(self.client.current_map(), task_type)
        else {
            return Err(MoveCommandError::MapNotFound);
        };
        self.r#move(map.x, map.y)
    }

    fn move_to_closest_map_of_type(
        &self,
        r#type: MapContentType,
    ) -> Result<Arc<MapSchema>, MoveCommandError> {
        let current_map = self.current_map();
        if current_map.content_type_is(r#type) {
            return Ok(current_map);
        }
        let Some(map) = self
            .maps
            .closest_of_type_from(self.client.current_map(), r#type)
        else {
            return Err(MoveCommandError::MapNotFound);
        };
        self.r#move(map.x, map.y)
    }

    fn move_to_closest_map_with_content_code(
        &self,
        code: &str,
    ) -> Result<Arc<MapSchema>, MoveCommandError> {
        let current_map = self.current_map();
        if current_map.content_code_is(code) {
            return Ok(current_map);
        }
        let Some(map) = self
            .maps
            .closest_with_content_code_from(self.client.current_map(), code)
        else {
            return Err(MoveCommandError::MapNotFound);
        };
        self.r#move(map.x, map.y)
    }

    fn r#move(&self, x: i32, y: i32) -> Result<Arc<MapSchema>, MoveCommandError> {
        if self.client.position() == (x, y) {
            return Ok(self.client.current_map());
        }
        Ok(self.client.r#move(x, y)?)
    }

    fn eat_food_from_inventory(&self) {
        self.inventory
            .consumable_food()
            .iter()
            .sorted_by_key(|i| i.heal())
            .for_each(|f| {
                // TODO: improve logic to eat different foods to restore more hp
                let mut quantity = self.missing_hp() / f.heal();
                if self.account.time_to_get(&f.code).is_some_and(|t| {
                    t * (self.missing_hp() / f.heal()) < Simulator::time_to_rest(self.missing_hp())
                }) {
                    quantity += 1;
                };
                if quantity > 0 {
                    quantity = min(quantity, self.inventory.total_of(&f.code));
                    if let Err(e) = self.use_item(&f.code, quantity) {
                        error!("{} failed to use food: {:?}", self.name(), e)
                    }
                }
            });
    }

    fn use_item(&self, item_code: &str, quantity: i32) -> Result<(), UseItemCommandError> {
        self.client.r#use(item_code, quantity)?;
        self.inventory.decrease_reservation(item_code, quantity);
        Ok(())
    }

    fn buy_item(&self, item_code: &str, quantity: i32) -> Result<(), BuyNpcCommandError> {
        let Some(npc_item) = self.npcs.items.get(item_code) else {
            return Err(BuyNpcCommandError::ItemNotPurchasable);
        };
        let Some(buy_price) = npc_item.buy_price else {
            return Err(BuyNpcCommandError::ItemNotPurchasable);
        };
        let total_price = buy_price * quantity;
        if self.has_available(&npc_item.currency) < total_price {
            return Err(BuyNpcCommandError::InsufficientCurrency);
        }
        if npc_item.currency == "gold" {
            let missing_quantity = total_price - self.gold();
            if missing_quantity > 0 {
                self.withdraw_gold(missing_quantity)?;
            }
        } else {
            let missing_quantity = total_price - self.inventory.total_of(item_code);
            if missing_quantity > 0 {
                self.deposit_all_but(item_code)?;
                self.withdraw_item(item_code, missing_quantity)?;
            }
        }
        self.move_to_closest_map_with_content_code(&npc_item.npc)?;
        self.client.npc_buy(item_code, quantity)?;
        Ok(())
    }

    /// TODO: improve with only ordering food crafted from fishing
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
                    self.name(),
                    e
                )
            }
        });
        if let Some(best_food) = self
            .items
            .best_consumable_foods(self.level())
            .iter()
            .max_by_key(|i| {
                self.account
                    .time_to_get(&i.code)
                    .map(|t| i.heal() / t)
                    .unwrap_or(0)
            })
            && self.bank.has_available(&best_food.code, None) < MIN_FOOD_THRESHOLD
            && let Err(e) = self.order_board.add_or_reset(
                &best_food.code,
                self.account.fisher_max_items(),
                Some(&self.name()),
                Purpose::Food {
                    char: self.name().to_owned(),
                },
            )
        {
            error!("{} failed to add or reset food order: {:?}", self.name(), e)
        }
    }

    fn order_gear(&self, gear: &mut Gear) {
        gear.align_to(&self.client.gear());
        Slot::iter().for_each(|s| {
            if !s.is_artifact_1()
                && !s.is_artifact_2()
                && !s.is_artifact_3()
                && !s.is_ring_1()
                && !s.is_ring_2()
                && let Some(item) = gear.slot(s)
            {
                let quantity = if s.is_utility_1() || s.is_utility_2() {
                    100
                } else {
                    1
                };
                self.order_if_needed(s, &item.code, quantity);
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
        if (self.equiped_in(slot).is_empty()
            || self
                .items
                .get(&self.equiped_in(slot))
                .is_some_and(|equiped| item != equiped.code))
            && self.has_in_bank_or_inv(item) < quantity
        {
            return self
                .order_board
                .add(
                    item,
                    quantity - self.has_available(item),
                    None,
                    Purpose::Gear {
                        char: self.name().to_owned(),
                        slot,
                        item_code: item.to_owned(),
                    },
                )
                .is_ok();
        }
        false
    }

    fn reserv_gear(&self, gear: &mut Gear) {
        gear.align_to(&self.client.gear());
        Slot::iter().for_each(|s| {
            if !(s.is_ring_1() || s.is_ring_2())
                && let Some(item) = gear.slot(s)
            {
                let quantity = if s.is_utility_1() || s.is_utility_2() {
                    100
                } else {
                    1
                };
                self.reserv_if_needed_and_available(s, &item.code, quantity);
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
        if (self.equiped_in(s).is_empty()
            || self
                .items
                .get(&self.equiped_in(s))
                .is_some_and(|equiped| item != equiped.code))
            && self.inventory.total_of(item) < quantity
            && let Err(e) =
                self.bank
                    .reserv(item, quantity - self.inventory.total_of(item), &self.name())
        {
            error!("{} failed to reserv '{}': {:?}", self.name(), item, e)
        }
    }

    #[allow(dead_code)]
    fn time_to_get_gear(&self, gear: &Gear) -> Option<i32> {
        Slot::iter()
            .map(|s| gear.slot(s).and_then(|i| self.items.time_to_get(&i.code)))
            .sum()
    }

    #[allow(dead_code)]
    pub fn time_to_get(&self, item: &str) -> Option<i32> {
        self.items
            .best_source_of(item)
            .iter()
            .filter_map(|s| match s {
                ItemSource::Resource(r) => self.time_to_gather(r),
                ItemSource::Monster(m) => self
                    .time_to_kill(m)
                    .map(|time| time * self.items.drop_rate(item)),
                ItemSource::Craft => Some(
                    CRAFT_TIME
                        + self
                            .items
                            .mats_of(item)
                            .iter()
                            .map(|m| self.time_to_get(&m.code).unwrap_or(1000) * m.quantity)
                            .sum::<i32>(),
                ),
                ItemSource::TaskReward => Some(2000),
                ItemSource::Task => Some(2000),
                ItemSource::Npc(_) => Some(60),
            })
            .min()
    }

    pub fn time_to_kill(&self, monster: &MonsterSchema) -> Option<i32> {
        let gear = self.can_kill(monster).ok()?;
        let fight = Simulator::fight(self.level(), 0, &gear, monster, false);
        Some(fight.cd + (fight.hp_lost / 5 + if fight.hp_lost % 5 > 0 { 1 } else { 0 }))
    }

    pub fn time_to_gather(&self, resource: &ResourceSchema) -> Option<i32> {
        self.can_gather(resource).ok()?;
        let tool = self.gear_finder.best_tool(
            self,
            resource.skill.into(),
            Filter {
                available: true,
                ..Default::default()
            },
        );
        let time = Simulator::gather_cd(
            resource.level,
            tool.map_or(0, |t| t.skill_cooldown_reduction(resource.skill.into())),
        );
        Some(time)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on
    /// inventory max items
    pub fn max_craftable_items(&self, item: &str) -> i32 {
        self.inventory.max_items() / self.items.mats_quantity_for(item)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory max items and bank materials.
    fn max_craftable_items_from_bank(&self, item: &str) -> i32 {
        min(
            self.bank.has_mats_for(item, Some(&self.name())),
            self.inventory.max_items() / self.items.mats_quantity_for(item),
        )
    }

    /// Returns the amount of the given item `code` available in bank, inventory and gear.
    pub fn has_available(&self, item: &str) -> i32 {
        self.has_equiped(item) as i32 + self.has_in_bank_or_inv(item)
    }

    /// Returns the amount of the given item `code` available in bank and inventory.
    fn has_in_bank_or_inv(&self, item: &str) -> i32 {
        self.inventory.total_of(item) + self.bank.has_available(item, Some(&self.name()))
    }

    /// Checks if the given item `code` is equiped.
    fn has_equiped(&self, item: &str) -> usize {
        Slot::iter()
            .filter(|s| {
                self.items
                    .get(&self.equiped_in(*s))
                    .is_some_and(|e| e.code == item)
            })
            .count()
    }

    pub fn skill_enabled(&self, s: Skill) -> bool {
        self.conf().read().unwrap().skills.contains(&s)
    }

    pub fn current_map(&self) -> Arc<MapSchema> {
        self.client.current_map()
    }

    pub fn toggle_idle(&self) {
        let mut conf = self.conf().write().unwrap();
        conf.idle ^= true;
        info!("{} toggled idle: {}.", self.name(), conf.idle);
        if !conf.idle {
            self.client.refresh_data()
        }
    }

    pub fn conf(&self) -> &RwLock<CharConfig> {
        self.config.characters.get(self.client.id).unwrap()
    }
}

impl HasCharacterData for CharacterController {
    fn data(&self) -> Arc<CharacterSchema> {
        self.client.data().clone()
    }

    fn server(&self) -> Arc<Server> {
        todo!()
    }

    fn refresh_data(&self) {
        self.client.refresh_data();
    }

    fn update_data(&self, schema: CharacterSchema) {
        self.client.update_data(schema);
    }
}
