use crate::{
    MIN_COIN_THRESHOLD, MIN_FOOD_THRESHOLD,
    account::AccountController,
    bank::Bank,
    bot_config::{BotConfig, CharConfig, Goal},
    error::{
        BankCleanupError, BankExpansionCommandError, BuyNpcCommandError,
        BuyNpcOrderProgressionError, CombatLevelingError, CraftCommandError,
        CraftOrderProgressionError, CraftSkillLevelingError, DeleteCommandError,
        DepositItemCommandError, EquipCommandError, GatherCommandError, GoldDepositCommandError,
        GoldWithdrawCommandError, KillMonsterCommandError, MoveCommandError, OrderDepositError,
        OrderProgressionError, RecycleCommandError, SellNpcCommandError, SkillLevelingError,
        TaskAcceptationCommandError, TaskCancellationCommandError, TaskCompletionCommandError,
        TaskProgressionError, TaskTradeCommandError, TasksCoinExchangeCommandError,
        TasksCoinExchangeOrderProgressionError, UnequipCommandError, UseItemCommandError,
        WithdrawItemCommandError,
    },
    gear_finder::{Filter, GearFinder},
    inventory::Inventory,
    leveling_helper::LevelingHelper,
    orderboard::{Order, OrderBoard, Purpose},
};
use anyhow::Result;
use artifactsmmo_sdk::{
    GOLDEN_EGG, GOLDEN_SHRIMP, HasDrops, Items, Maps, Monsters, Server, Simulator,
    char::{Character as CharacterClient, HasCharacterData, Skill, error::RestError},
    consts::{
        BANK_MIN_FREE_SLOT, CRAFT_TIME, GOLD, MAX_LEVEL, TASK_CANCEL_PRICE, TASK_EXCHANGE_PRICE,
        TASKS_COIN,
    },
    gear::{Gear, Slot},
    items::{ItemSchemaExt, ItemSource},
    maps::MapSchemaExt,
    models::{
        CharacterSchema, DropSchema, FightResult, FightSchema, ItemSchema, MapContentType,
        MapSchema, MonsterSchema, NpcItem, RecyclingItemsSchema, ResourceSchema, RewardsSchema,
        SimpleItemSchema, SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, TaskType,
    },
    monsters::MonsterSchemaExt,
    npcs::Npcs,
    resources::ResourceSchemaExt,
    simulator::HasEffects,
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
            inventory: Arc::new(Inventory::new(client.clone(), items.clone())),
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
                    error!("{}: failed depositing in main loop: {e}", self.name())
                }
                continue;
            }
            self.maps.refresh_from_events();
            self.order_food();
            if self.cleanup_bank().is_ok() {
                continue;
            }
            if self.handle_goals() {
                continue;
            }
            // TODO: improve fallback
            match self.progress_task() {
                Ok(_) => continue,
                Err(TaskProgressionError::TaskTradeCommandError(
                    TaskTradeCommandError::MissingItems { item, quantity },
                )) => {
                    if self
                        .order_board
                        .add(
                            &item,
                            quantity,
                            Some(&self.name()),
                            Purpose::Task {
                                char: self.name().to_owned(),
                            },
                        )
                        .is_ok()
                    {
                        continue;
                    }
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
        match self.level_skill_by_crafting(skill).is_ok() {
            true => Ok(()),
            false => Ok(self.level_skill_by_gathering(skill)?),
        }
    }

    /// Find a target and kill it if possible.
    fn level_combat(&self) -> Result<(), CombatLevelingError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(KillMonsterCommandError::SkillDisabled(Skill::Combat).into());
        }
        if self.task_type().is_some_and(|t| t == TaskType::Monsters) {
            return Ok(self.progress_task()?).map(|_| ());
        }
        let Some(monster) = self.leveling_helper.best_monster(self) else {
            return Err(CombatLevelingError::NoMonsterFound);
        };
        self.kill_monster(&monster)?;
        Ok(())
    }

    fn level_skill_by_crafting(&self, skill: Skill) -> Result<(), CraftSkillLevelingError> {
        let Some(item) = self
            .leveling_helper
            .best_craft(self.skill_level(skill), skill, self)
        else {
            return Err(CraftSkillLevelingError::ItemNotFound);
        };
        let quantity = self.max_craftable_items(&item.code);
        match self.craft_from_bank(&item.code, quantity) {
            Ok(_) => {
                if !(skill.is_gathering() || skill.is_cooking())
                    && let Err(e) = self.recycle_item(&item.code, quantity)
                {
                    error!(
                        "{}: failed recycling crafted items for leveling: {e}",
                        self.name()
                    )
                };
                Ok(())
            }
            Err(CraftCommandError::InsufficientMaterials(missing_mats))
                if !skill.is_gathering()
                    || skill.is_alchemy()
                        && self
                            .leveling_helper
                            .best_resource(self.skill_level(skill), skill)
                            .is_none() =>
            {
                Ok(self.order_board.add_multiple(
                    missing_mats,
                    None,
                    &Purpose::Leveling {
                        char: self.name().to_owned(),
                        skill,
                    },
                )?)
            }
            Err(e) => Err(e.into()),
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

    /// Browse orderboard for completable orders: first check if some orders
    /// can be turned in, then check for completable orders (enough materials to craft all items
    /// from an order. Then check for orders that can be progressed. Then check for order for which
    /// the skill level required needs to be leveled.
    fn handle_orderboard(&self) -> bool {
        let orders = self.order_board.orders_by_priority();
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

    fn handle_order(&self, order: Arc<Order>) -> Result<i32, OrderProgressionError> {
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

    /// Checks if the character is able to get the missing items for the `order` in one command
    /// Resource and Monsters sources return false because drop rate might not be 100%
    /// TODO: maybe check drop rate of item and return `true` if it is 100%
    fn can_complete_order(&self, order: &Order) -> bool {
        let missing = self.order_board.total_missing_for(order);
        self.items
            .best_source_of(&order.item)
            .iter()
            .any(|s| match s {
                ItemSource::Resource(_) => false,
                ItemSource::Monster(_) => false,
                ItemSource::Craft => self
                    .can_craft_now(
                        &order.item,
                        min(missing, self.max_craftable_items(&order.item)),
                    )
                    .is_ok(),
                ItemSource::TaskReward => self.can_exchange_task().is_ok(),
                ItemSource::Task => self.has_available(&self.task()) >= self.task_missing(),
                ItemSource::Npc(_) => self.can_buy_item(&order.item, missing).is_ok(),
            })
    }

    fn progress_order(&self, order: &Order) -> Result<i32, OrderProgressionError> {
        if self.order_board.total_missing_for(order) <= 0 {
            return Err(OrderProgressionError::NoItemMissing);
        }
        let Some(source) = self.items.best_source_of(&order.item) else {
            return Err(OrderProgressionError::NoSourceForItem);
        };
        Ok(match source {
            ItemSource::Resource(r) => self.progress_resource_order(order, &r)?,
            ItemSource::Monster(m) => self.progress_monster_order(order, &m)?,
            ItemSource::Craft => self.progress_crafting_order(order)?,
            ItemSource::TaskReward => self.progress_task_reward_order(order)?,
            ItemSource::Task => self.progress_task_order(order)?,
            ItemSource::Npc(_) => self.progress_buy_npc_order(order)?,
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

    fn progress_crafting_order(&self, order: &Order) -> Result<i32, CraftOrderProgressionError> {
        let total_missing = self.order_board.total_missing_for(order);
        match self.can_craft_now(&order.item, total_missing) {
            Ok(_) => {
                let quantity = min(total_missing, self.max_craftable_items(&order.item));
                order.inc_in_progress(quantity);
                let result = self.craft_from_bank(&order.item, quantity);
                order.dec_in_progress(quantity);
                Ok(result.map(|craft| craft.amount_of(&order.item))?)
            }
            Err(CraftCommandError::InsufficientMaterials(missing_mats)) => Ok(self
                .order_board
                .add_multiple(missing_mats, None, &order.purpose)
                .map(|_| 0)?),
            Err(e) => Err(e.into()),
        }
    }

    fn progress_task_reward_order(
        &self,
        order: &Order,
    ) -> Result<i32, TasksCoinExchangeOrderProgressionError> {
        match self.can_exchange_task() {
            Ok(()) => {
                order.inc_in_progress(1);
                let exchanged = self.exchange_task().map(|r| r.amount_of(&order.item));
                order.dec_in_progress(1);
                Ok(exchanged?)
            }
            Err(TasksCoinExchangeCommandError::InsufficientCoins(quantity)) => {
                self.order_board
                    .add(TASKS_COIN, quantity, None, order.purpose.to_owned())?;
                Ok(0)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn progress_task_order(&self, order: &Order) -> Result<i32, TaskProgressionError> {
        match self.progress_task() {
            Ok(r) => Ok(r.amount_of(&order.item)),
            Err(TaskProgressionError::TaskTradeCommandError(
                TaskTradeCommandError::MissingItems { item, quantity },
            )) => {
                self.order_board
                    .add(&item, quantity, Some(&self.name()), order.purpose.clone())?;
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    fn progress_buy_npc_order(&self, order: &Order) -> Result<i32, BuyNpcOrderProgressionError> {
        let total_missing = self.order_board.total_missing_for(order);
        match self.can_buy_item(&order.item, total_missing) {
            Ok(_) => {
                order.inc_in_progress(total_missing);
                let purchase = self
                    .buy_item(&order.item, total_missing)
                    .map(|_| total_missing);
                order.dec_in_progress(total_missing);
                Ok(purchase?)
            }
            Err(BuyNpcCommandError::InsufficientCurrency { currency, quantity }) => {
                if currency != GOLD {
                    self.order_board
                        .add(&currency, quantity, None, order.purpose.clone())?;
                }
                Ok(0)
            }
            Err(e) => Err(e.into()),
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

    fn progress_task(&self) -> Result<Vec<DropSchema>, TaskProgressionError> {
        if self.task().is_empty() {
            let r#type = self.conf().read().unwrap().task_type;
            return Ok(self.accept_task(r#type).map(|_| vec![])?);
        }
        if self.task_finished() {
            return Ok(self.complete_task().map(|i| {
                i.items
                    .iter()
                    .map(|i| DropSchema {
                        code: i.code.clone(),
                        quantity: i.quantity,
                    })
                    .collect()
            })?);
        }
        let Some(monster) = self.monsters.get(&self.task()) else {
            return Ok(self.trade_task().map(|r| {
                vec![DropSchema {
                    code: r.code,
                    quantity: r.quantity,
                }]
            })?);
        };
        match self.kill_monster(&monster) {
            Ok(r) => Ok(r.drops),
            Err(KillMonsterCommandError::GearTooWeak { monster_code }) => {
                warn!(
                    "{}: No gear powerfull enough to kill {}",
                    self.name(),
                    monster_code
                );
                self.cancel_task()?;
                Ok(vec![])
            }
            Err(e) => Err(e.into()),
        }
    }

    fn trade_task(&self) -> Result<TaskTradeSchema, TaskTradeCommandError> {
        self.can_trade_task()?;
        let q = min(self.task_missing(), self.inventory.max_items());
        if let Err(e) = self.bank.reserv_item(&self.task(), q, &self.name()) {
            error!(
                "{}: failed reserving items for task trade: {e}",
                self.name()
            )
        }
        if let Err(e) = self.deposit_all() {
            error!(
                "{}: failed depositing all before task trading: {e}",
                self.name(),
            )
        }
        if let Err(e) = self.withdraw_item(&self.task(), q) {
            error!(
                "{}: failed withdrawing item for task trading: {e}",
                self.name(),
            );
            self.bank.unreserv_item(&self.task(), q, &self.name());
        };
        self.move_to_closest_taskmaster(self.task_type())?;
        let res = self.client.task_trade(&self.task(), q);
        self.inventory.unreserv_item(&self.task(), q);
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

        let missing_quantity = self.task_missing() - self.has_in_bank_or_inv(&self.task());
        if missing_quantity > 0 {
            return Err(TaskTradeCommandError::MissingItems {
                item: self.task().to_owned(),
                quantity: missing_quantity,
            });
        }
        Ok(())
    }

    fn accept_task(&self, r#type: TaskType) -> Result<TaskSchema, TaskAcceptationCommandError> {
        if !self.task().is_empty() {
            return Err(TaskAcceptationCommandError::TaskAlreadyInProgress);
        }
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
        let coin_available = self.has_in_bank_or_inv(TASKS_COIN);
        let min_coins = TASK_EXCHANGE_PRICE + MIN_COIN_THRESHOLD;
        if coin_available < min_coins {
            return Err(TasksCoinExchangeCommandError::InsufficientCoins(
                min_coins
                    - if self.order_board.is_ordered(TASKS_COIN) {
                        0
                    } else {
                        coin_available
                    },
            ));
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
                .reserv_item(TASKS_COIN, self.inventory.total_of(TASKS_COIN))
            {
                error!(
                    "{}: failed reserving tasks coins already in inventory: {e}",
                    self.name(),
                );
            }
        } else {
            //TODO: should add variant for reservation error, or reservation should be handled in
            //`withdraw_item` method
            if self
                .bank
                .reserv_item(TASKS_COIN, quantity, &self.name())
                .is_err()
            {
                return Err(TasksCoinExchangeCommandError::InsufficientCoins(quantity));
            }
            if let Err(e) = self.deposit_all_but(TASKS_COIN) {
                error!(
                    "{}: failed to deposit all while exchanging task: {e}",
                    self.name()
                )
            }
            self.withdraw_item(TASKS_COIN, quantity)?;
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        let result = self.client.exchange_tasks_coin().map_err(|e| e.into());
        self.inventory
            .unreserv_item(TASKS_COIN, TASK_EXCHANGE_PRICE);
        result
    }

    fn cancel_task(&self) -> Result<(), TaskCancellationCommandError> {
        if self.bank.has_available(TASKS_COIN, Some(&self.name()))
            < TASK_CANCEL_PRICE + MIN_COIN_THRESHOLD
        {
            return Err(TaskCancellationCommandError::NotEnoughCoins);
        }
        if self.inventory.has_available(TASKS_COIN) <= 0 {
            if self
                .bank
                .reserv_item(TASKS_COIN, TASK_CANCEL_PRICE, &self.name())
                .is_err()
            {
                return Err(TaskCancellationCommandError::NotEnoughCoins);
            }
            if let Err(e) = self.deposit_all() {
                error!(
                    "{}: failed depositing while canceling task: {e}",
                    self.name()
                )
            }
            self.withdraw_item(TASKS_COIN, TASK_CANCEL_PRICE)?;
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        let result = self.client.cancel_task().map_err(|e| e.into());
        self.inventory.unreserv_item(TASKS_COIN, TASK_CANCEL_PRICE);
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
            error!("{}: failed accepting new task: {e}", self.name())
        }
        if self.inventory.free_space() < monster.max_drop_quantity()
            || self
                .current_map()
                .monster()
                .is_none_or(|m| m != monster.code)
        {
            self.deposit_all()?;
        };
        self.check_for_combat_gear(monster)?;
        self.withdraw_food();
        if !self.can_kill_now(monster) || self.health() < 10 {
            self.eat_food_from_inventory();
        }
        if (!self.can_kill_now(monster) || self.health() < 10)
            && let Err(e) = self.rest()
        {
            error!("{}: failed to rest: {e}", self.name())
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
        drop(_browsed);
        if self.config.order_gear {
            self.order_best_gear_against(monster);
        }
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
        self.can_gather_now(resource)?;
        if self.inventory.free_space() < resource.max_drop_quantity()
            || self
                .current_map()
                .resource()
                .is_none_or(|r| r != resource.code)
        {
            self.deposit_all()?;
        };
        self.check_for_skill_gear(resource.skill.into());
        self.move_to_closest_map_with_content_code(&resource.code)?;
        Ok(self.client.gather()?)
    }

    fn can_gather_now(&self, resource: &ResourceSchema) -> Result<(), GatherCommandError> {
        self.can_gather(resource)?;
        if self.maps.with_content_code(&resource.code).is_empty() {
            return Err(GatherCommandError::MapNotFound);
        };
        Ok(())
    }

    // Checks that the `Character` has the required skill level to gather the given `resource`
    fn can_gather(&self, resource: &ResourceSchema) -> Result<(), GatherCommandError> {
        let skill: Skill = resource.skill.into();
        if !self.skill_enabled(skill) {
            return Err(GatherCommandError::SkillDisabled(skill));
        }
        if self.skill_level(skill) < resource.level {
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
        drop(_browsed);
        if self.config.order_gear {
            self.order_best_gear_for_skill(skill);
        }
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
        self.order_gear(&mut gear)
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
        (1..=100)
            .map(|_| Simulator::random_fight(self.client.level(), 0, gear, monster, false).result)
            .filter(|r| *r == FightResult::Win)
            .count()
            >= 95
    }

    fn can_kill_now(&self, monster: &MonsterSchema) -> bool {
        Simulator::average_fight(
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
        let (item, skill) = self.can_craft_now(item, quantity)?;
        info!(
            "{}: going to craft '{}'x{} from bank.",
            self.name(),
            item.code,
            quantity
        );
        let mats = self.items.mats_for(&item.code, quantity);
        if let Err(e) = self.bank.reserv_items(&mats, &self.name()) {
            error!(
                "{}: failed reserving mats to craft from bank: {e}",
                self.name(),
            )
        };
        self.check_for_skill_gear(skill);
        self.deposit_all()?;
        self.withdraw_items(&mats)?;
        let Some(map) = self.maps.with_workshop_for(skill) else {
            return Err(MoveCommandError::MapNotFound.into());
        };
        self.r#move(map.x, map.y)?;
        let craft = self.client.craft(&item.code, quantity)?;
        mats.iter().for_each(|m| {
            self.inventory.unreserv_item(&m.code, m.quantity);
        });
        Ok(craft)
    }

    // Checks that the `Character` has the required skill level to craft the given item `code`
    pub fn can_craft_now(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<(Arc<ItemSchema>, Skill), CraftCommandError> {
        let (item, skill) = self.can_craft(item)?;
        let missing_mats = self.missing_mats_for(&item.code, quantity);
        if !missing_mats.is_empty() {
            return Err(CraftCommandError::InsufficientMaterials(missing_mats));
        }
        if self.max_craftable_items(&item.code) < quantity {
            return Err(CraftCommandError::InsufficientInventorySpace);
        }
        Ok((item, skill))
    }

    pub fn can_craft(&self, item: &str) -> Result<(Arc<ItemSchema>, Skill), CraftCommandError> {
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
        Ok((item, skill))
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
            if let Err(e) = self
                .bank
                .reserv_item(&item.code, missing_quantity, &self.name())
            {
                error!("{}: failed reserving before recyling: {e}", self.name());
            }
            if let Err(e) = self.deposit_all_but(&item.code) {
                error!("{}: failed depositing before recycling: {e}", &self.name(),)
            }
            self.withdraw_item(&item.code, missing_quantity)?;
        }
        let Some(map) = self.maps.with_workshop_for(skill) else {
            return Err(MoveCommandError::MapNotFound.into());
        };
        self.r#move(map.x, map.y)?;
        let result = self.client.recycle(&item.code, quantity);
        self.inventory.unreserv_item(&self.task(), quantity);
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
        info!("{}: going to delete '{}'x{}.", self.name(), item, quantity);
        if self.inventory.has_available(item) < quantity {
            let missing_quantity = quantity - self.inventory.has_available(item);
            if let Err(e) = self.bank.reserv_item(item, missing_quantity, &self.name()) {
                error!(
                    "{}: failed reserving before item deletion: {e}",
                    self.name(),
                );
            }
            if let Err(e) = self.deposit_all_but(item) {
                error!(
                    "{}: failed depositing before item deletion: {e}",
                    self.name(),
                )
            }
            self.withdraw_item(item, missing_quantity)?;
        }
        let result = self.client.delete(item, quantity);
        self.inventory.unreserv_item(&self.task(), quantity);
        Ok(result?)
    }

    /// Deposits all the gold and items in the character inventory into the bank.
    /// Items needed by orders are turned in first.
    pub fn deposit_all(&self) -> Result<(), DepositItemCommandError> {
        if self.inventory.total_items() <= 0 {
            return Ok(());
        }
        self.deposit_items(&self.inventory.simple_content(), None)
    }

    pub fn deposit_all_but(&self, item: &str) -> Result<(), DepositItemCommandError> {
        if self.inventory.total_items() <= 0 {
            return Ok(());
        }
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
        info!("{}: going to deposit items: {items:?}", self.name(),);
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        if self.bank.free_slots() <= BANK_MIN_FREE_SLOT
            && let Err(e) = self.expand_bank()
        {
            error!("{}: failed to expand bank capacity: {e}", self.name())
        }
        let deposit = self.client.deposit_item(items);
        match deposit {
            Ok(_) => {
                self.order_board.register_deposited_items(items, &owner);
                if let Some(ref owner) = owner {
                    items.iter().for_each(|i| {
                        if let Err(e) = self.bank.increase_reservation(&i.code, i.quantity, owner) {
                            error!("{}: failed reserving deposited item: {e}", self.name())
                        }
                    })
                }
                items.iter().for_each(|i| {
                    self.inventory.unreserv_item(&i.code, i.quantity);
                });
            }
            Err(ref e) => error!("{}: error depositing: {e}", self.name()),
        }
        if let Err(e) = self.deposit_all_gold() {
            error!("{}: failed to deposit gold to the bank: {e}", self.name(),)
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
        if let Err(e) = self.bank.reserv_item(&food.code, quantity, &self.name()) {
            error!("{}: failed reserving food: {e}", self.name())
        };
        drop(_browsed);
        // TODO: only deposit what is necessary, food already in inventory should be kept
        if let Err(e) = self.deposit_all() {
            error!(
                "{}: failed depositing before withdrawing food: {e}",
                self.name(),
            )
        }
        if let Err(e) = self.withdraw_item(&food.code, quantity) {
            error!("{}: failed withdrawing food: {e}", self.name())
        }
    }

    pub fn withdraw_item(&self, item: &str, quantity: i32) -> Result<(), WithdrawItemCommandError> {
        self.withdraw_items(&[SimpleItemSchema {
            code: item.into(),
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
        info!("{}: going to withdraw items: {items:?}", self.name());
        self.move_to_closest_map_of_type(MapContentType::Bank)?;
        let result = self.client.withdraw_item(items);
        if result.is_ok() {
            self.bank.unreserv_items(items, &self.name());
            if let Err(e) = self.inventory.reserv_items(items) {
                error!("{}: failed reserving withdrawed item: {e}", self.name());
            }
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
            if let Err(e) = self.withdraw_item(item, quantity) {
                error!("{} failed withdrawing item for equiping: {e}", self.name(),);
            }
        }
        if let Err(e) = self.equip_item(
            item,
            slot,
            min(slot.max_quantity(), self.inventory.total_of(item)),
        ) {
            error!(
                "{} failed equiping item from bank or inventory: {e}",
                self.name(),
            );
        }
        if let Some(i) = prev_equiped
            && self.inventory.total_of(&i.code) > 0
            && let Err(e) = self.deposit_item(&i.code, self.inventory.total_of(&i.code), None)
        {
            error!(
                "{} failed depositing item previously equiped: {e}",
                self.name(),
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
                "{}: failed depositing before equiping item: {e}",
                self.name()
            )
        }
        self.unequip_slot(slot, self.quantity_in_slot(slot))?;
        self.client.equip(item_code, slot, quantity)?;
        self.inventory.unreserv_item(item_code, quantity);
        Ok(())
    }

    pub fn unequip_and_deposit_all(&self) {
        Slot::iter().for_each(|s| {
            if let Some(item) = self.items.get(&self.equiped_in(s)) {
                let quantity = self.quantity_in_slot(s);
                if let Err(e) = self.unequip_slot(s, quantity) {
                    error!(
                        "{}: failed to unequip '{}'x{quantity} during unequip_and_deposit_all: {e}",
                        self.name(),
                        &item.code,
                    )
                } else if let Err(e) = self.deposit_item(&item.code, quantity, None) {
                    error!(
                        "{}: failed to deposit '{}'x{quantity} during `unequip_and_deposit_all`: {e}",
                        self.name(),
                        &item.code,
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
        self.inventory.unreserv_item(item_code, quantity);
        Ok(())
    }

    fn buy_item(&self, item_code: &str, quantity: i32) -> Result<(), BuyNpcCommandError> {
        let (npc_item, total_price) = self.can_buy_item(item_code, quantity)?;
        if npc_item.currency == GOLD {
            let missing_quantity = total_price - self.gold();
            if missing_quantity > 0 {
                self.withdraw_gold(missing_quantity)?;
            }
        } else {
            let missing_quantity = total_price - self.inventory.total_of(&npc_item.currency);
            if let Err(e) =
                self.bank
                    .reserv_item(&npc_item.currency, missing_quantity, &self.name())
            {
                error!(
                    "{}: failed reserving currency in bank for purchase: {e}",
                    self.name(),
                )
            }
            if let Err(e) = self.deposit_all_but(&npc_item.currency) {
                error!("{}: failed depositing  before purchase: {e}", self.name(),)
            }
            self.withdraw_item(&npc_item.currency, missing_quantity)?;
            if let Err(e) = self.inventory.reserv_item(&npc_item.currency, total_price) {
                error!(
                    "{}: failed reserving currency in inventory for purchase: {e}",
                    self.name()
                );
            }
        }
        self.move_to_closest_map_with_content_code(&npc_item.npc)?;
        self.client.npc_buy(item_code, quantity)?;
        self.inventory
            .unreserv_item(&npc_item.currency, total_price);
        Ok(())
    }

    fn can_buy_item(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<(Arc<NpcItem>, i32), BuyNpcCommandError> {
        let Some(npc_item) = self.npcs.items.get(item_code) else {
            return Err(BuyNpcCommandError::ItemNotFound(item_code.to_string()));
        };
        let Some(buy_price) = npc_item.buy_price else {
            return Err(BuyNpcCommandError::ItemNotPurchasable);
        };
        let total_price = buy_price * quantity;
        let currency_available = if npc_item.currency == GOLD {
            self.gold() + self.bank.gold()
        } else {
            self.has_in_bank_or_inv(&npc_item.currency)
        };
        if currency_available < total_price {
            return Err(BuyNpcCommandError::InsufficientCurrency {
                currency: npc_item.currency.to_string(),
                quantity: total_price
                    - if self.order_board.is_ordered(&npc_item.currency) {
                        0
                    } else {
                        currency_available
                    },
            });
        }
        if self.maps.with_content_code(&npc_item.npc).is_empty() {
            return Err(BuyNpcCommandError::NpcNotFound);
        }
        Ok((npc_item, total_price))
    }

    fn sell_item(&self, item_code: &str, quantity: i32) -> Result<(), SellNpcCommandError> {
        let npc_item = self.can_sell_item(item_code, quantity)?;
        let missing_quantity = quantity - self.inventory.total_of(&npc_item.currency);
        //TODO: reserv item already in inventory
        if let Err(e) = self
            .bank
            .reserv_item(item_code, missing_quantity, &self.name())
        {
            error!(
                "{}: failed reserving item from bank for selling: {e}",
                self.name(),
            )
        }
        if missing_quantity > 0 {
            self.deposit_all_but(item_code)?;
            self.withdraw_item(item_code, missing_quantity)?;
            if let Err(e) = self.inventory.reserv_item(item_code, quantity) {
                error!(
                    "{}: failed reserving item from inventory for selling: {e}",
                    self.name(),
                );
            }
        }
        self.move_to_closest_map_with_content_code(&npc_item.npc)?;
        self.client.npc_sell(item_code, quantity)?;
        self.inventory.unreserv_item(item_code, quantity);
        Ok(())
    }

    fn can_sell_item(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<Arc<NpcItem>, SellNpcCommandError> {
        let Some(npc_item) = self.npcs.items.get(item_code) else {
            return Err(SellNpcCommandError::ItemNotFound(item_code.to_string()));
        };
        if npc_item.sell_price.is_none() {
            return Err(SellNpcCommandError::ItemNotSellable);
        };
        let available = self.has_in_bank_or_inv(item_code);
        if available < quantity {
            return Err(SellNpcCommandError::InsufficientQuantity {
                quantity: quantity - available,
            });
        }
        if self.maps.with_content_code(&npc_item.npc).is_empty() {
            return Err(SellNpcCommandError::NpcNotFound);
        }
        Ok(npc_item)
    }

    /// TODO: improve with only ordering food crafted from fishing
    fn order_food(&self) {
        if !self.skill_enabled(Skill::Combat) {
            return;
        }
        self.inventory.consumable_food().iter().for_each(|f| {
            if let Err(e) = self
                .inventory
                .reserv_item(&f.code, self.inventory.total_of(&f.code))
            {
                error!("{} failed reserving food in inventory: {e}", self.name(),)
            }
        });
        if let Some(best_food) = self
            .items
            .all()
            .iter()
            .filter(|i| i.is_food() && i.level <= self.level())
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
            error!("{} failed to add or reset food order: {e}", self.name())
        }
    }

    fn cleanup_bank(&self) -> Result<(), BankCleanupError> {
        if self.bank.content().iter().any(|i| {
            (i.code == GOLDEN_SHRIMP || i.code == GOLDEN_EGG)
                && self
                    .sell_item(
                        &i.code,
                        min(
                            self.bank.has_available(&i.code, Some(&self.name())),
                            self.inventory.max_items(),
                        ),
                    )
                    .is_ok()
        }) {
            Ok(())
        } else {
            Err(BankCleanupError::NoItemToHandle)
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
        if let Some(ref ring1) = gear.ring1
            && gear.ring1 == gear.ring2
        {
            self.order_if_needed(Slot::Ring1, &ring1.code, 2);
        } else {
            if let Some(ref ring1) = gear.ring1 {
                self.order_if_needed(Slot::Ring1, &ring1.code, 1);
            }
            if let Some(ref ring2) = gear.ring2 {
                self.order_if_needed(Slot::Ring2, &ring2.code, 1);
            }
        }
    }

    fn order_if_needed(&self, slot: Slot, item: &str, quantity: i32) -> bool {
        //TODO: prevent ordering item if the maximum quantity equipable by the whole account is
        //available(no more than 5 weapons, 10 rings, etc... utilities are exempt)
        if self.has_available(item) < quantity {
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
                    .reserv_item(item, quantity - self.inventory.total_of(item), &self.name())
        {
            error!("{}: failed reserving '{item}'x{quantity}: {e}", self.name())
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
        let fight = Simulator::average_fight(self.level(), 0, &gear, monster, false);
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

    pub fn gold_available(&self) -> i32 {
        self.gold() + self.bank.gold()
    }

    /// Returns the amount of the given item `code` available in bank, inventory and gear.
    pub fn has_available(&self, item: &str) -> i32 {
        self.has_equiped(item) + self.has_in_bank_or_inv(item)
    }

    /// Returns the amount of the given item `code` available in bank and inventory.
    fn has_in_bank_or_inv(&self, item: &str) -> i32 {
        self.inventory.total_of(item) + self.bank.has_available(item, Some(&self.name()))
    }

    fn missing_mats_for(&self, item_code: &str, quantity: i32) -> Vec<SimpleItemSchema> {
        self.items
            .mats_of(item_code)
            .into_iter()
            .filter(|m| self.has_in_bank_or_inv(&m.code) < m.quantity * quantity)
            .update(|m| {
                m.quantity = m.quantity * quantity
                    - if self.order_board.is_ordered(&m.code) {
                        0
                    } else {
                        self.has_in_bank_or_inv(&m.code)
                    }
            })
            .collect_vec()
    }

    pub fn current_map(&self) -> Arc<MapSchema> {
        self.client.current_map()
    }

    pub fn skill_enabled(&self, s: Skill) -> bool {
        self.conf().read().unwrap().skills.contains(&s)
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
}

impl HasCharacterData for CharacterController {
    fn data(&self) -> Arc<CharacterSchema> {
        self.client.data().clone()
    }

    fn server(&self) -> Arc<Server> {
        self.client.server()
    }

    fn refresh_data(&self) {
        self.client.refresh_data();
    }

    fn update_data(&self, schema: CharacterSchema) {
        self.client.update_data(schema);
    }
}
