use super::{
    account::Account,
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    bank::Bank,
    char_config::CharConfig,
    events::Events,
    fight_simulator::FightSimulator,
    game::{Game, Server},
    game_config::GameConfig,
    gear::{Gear, Slot},
    gear_finder::{Filter, GearFinder},
    inventory::Inventory,
    items::{ItemSource, Items, Type},
    maps::Maps,
    monsters::Monsters,
    orderboard::{Order, OrderBoard, Purpose},
    resources::Resources,
    skill::Skill,
    ActiveEventSchemaExt, FightSchemaExt, ItemSchemaExt, SkillSchemaExt, TaskRewardsSchemaExt,
};
use crate::artifactsmmo_sdk::{char_config::Goal, SkillInfoSchemaExt};
use actions::{PostCraftAction, RequestError};
use artifactsmmo_openapi::models::{
    CharacterSchema, FightResult, FightSchema, ItemSchema, MapContentSchema, MapSchema,
    MonsterSchema, RecyclingItemsSchema, ResourceSchema, SimpleItemSchema, SkillDataSchema,
    SkillInfoSchema, TaskRewardsSchema, TaskSchema, TaskType,
};
use itertools::Itertools;
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    cmp::min,
    io,
    option::Option,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
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
    server: Arc<Server>,
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
    pub inventory: Arc<Inventory>,
}

impl Character {
    pub fn new(
        config: &GameConfig,
        account: &Arc<Account>,
        game: &Game,
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
            server: game.server.clone(),
            maps: game.maps.clone(),
            resources: game.resources.clone(),
            monsters: game.monsters.clone(),
            items: game.items.clone(),
            events: game.events.clone(),
            orderboard: game.orderboard.clone(),
            gear_finder: GearFinder::new(&game.items),
            fight_simulator: FightSimulator::new(),
            bank: bank.clone(),
            data: data.clone(),
            inventory: Arc::new(Inventory::new(data)),
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
        loop {
            if self.conf.read().unwrap().idle {
                continue;
            }
            if self.inventory.is_full() {
                self.deposit_all();
                continue;
            }
            self.events.refresh();
            // TODO: improve the way ReachSkillLevel is handled
            let first_level_goal_not_reached = self.conf().goals.into_iter().find(|g| {
                if let Goal::ReachSkillLevel { skill, level } = g {
                    self.skill_level(*skill) < *level
                } else {
                    false
                }
            });
            if self
                .conf()
                .goals
                .iter()
                .filter(|g| {
                    g.is_reach_skill_level()
                        && first_level_goal_not_reached.is_some_and(|gnr| **g == gnr)
                        || !g.is_reach_skill_level()
                })
                .any(|g| match g {
                    Goal::Events => self.handle_events(),
                    Goal::Orders => self.handle_orderboard(),
                    Goal::ReachSkillLevel { skill, level } if self.skill_level(*skill) < *level => {
                        self.level_skill_up(*skill)
                    }
                    Goal::FollowMaxSkillLevel {
                        skill,
                        skill_to_follow,
                    } if self.skill_level(*skill)
                        < min(1 + self.account.max_skill_level(*skill_to_follow), 40) =>
                    {
                        self.level_skill_up(*skill)
                    }
                    _ => false,
                })
            {
                continue;
            }
            // TODO: improve fallback
            if let Err(e) = self.level_combat() {
                error!("{} failed to level combat: {:?}", self.name, e);
            }
        }
    }

    fn level_skill_up(&self, skill: Skill) -> bool {
        if skill.is_combat() {
            return self.level_combat().is_ok();
        }
        self.level_skill_by_crafting(skill).is_ok() || self.level_skill_by_gathering(&skill).is_ok()
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

    fn level_skill_by_crafting(&self, skill: Skill) -> Result<(), CharacterError> {
        let Some(item) = self
            .items
            .best_for_leveling_hc(self.skill_level(skill), skill)
            .into_iter()
            .min_by_key(|i| {
                self.bank.missing_mats_quantity(
                    &i.code,
                    self.max_craftable_items(&i.code),
                    Some(&self.name),
                )
            })
        else {
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
            if (!skill.is_gathering() || skill.is_alchemy())
                && self.order_missing_mats(
                    &item.code,
                    self.max_craftable_items(&item.code),
                    Purpose::Leveling {
                        char: self.name.to_owned(),
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
                self.name,
                &item.code,
                s.amount_of(&item.code)
            );
        })
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
                if order.in_progress() <= 0 {
                    // NOTE: Maybe ordering missing mats should be done elsewhere
                    self.order_missing_mats(
                        &order.item,
                        self.orderboard.total_missing(order),
                        order.purpose.clone(),
                    );
                };
                true
            }),
            ItemSource::TaskReward => order.in_progress() <= 0,
            ItemSource::Task => true,
        })
    }

    /// Creates orders based on the missing (not available in bank) materials requiered to craft
    /// the `quantity` of the given `item`. Orders are created with the given `priority` and
    /// `purpose`. Returns true if an order has been made.
    fn order_missing_mats(&self, item: &str, quantity: i32, purpose: Purpose) -> bool {
        let mut ordered: bool = false;
        self.items
            .mats(item)
            .into_iter()
            .filter(|m| self.bank.has_item(&m.code, Some(&self.name)) < m.quantity * quantity)
            .update(|m| {
                m.quantity = m.quantity * quantity
                    - if self.orderboard.is_ordered(&m.code) {
                        0
                    } else {
                        self.bank.has_item(&m.code, Some(&self.name))
                    }
            })
            .for_each(|m| {
                if self
                    .orderboard
                    .add(Order::new(None, &m.code, m.quantity, purpose.clone()))
                {
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
            ItemSource::TaskReward => self.has_available("tasks_coin") >= 6,
            ItemSource::Task => self.has_available(&self.task()) >= self.task_missing(),
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
                self.account.available_in_inventories(&order.item),
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
                ItemSource::Task => self.progress_task_order(order),
            })
    }

    fn progress_crafting_order(&self, order: &Order) -> Option<i32> {
        match self.can_craft(&order.item) {
            Ok(()) => {
                if self.orderboard.total_missing(order) <= 0 {
                    return None;
                }
                let quantity = min(
                    self.max_craftable_items(&order.item),
                    self.orderboard.total_missing(order),
                );
                if quantity <= 0 {
                    return None;
                }
                order.inc_in_progress(quantity);
                let crafted = self.craft_from_bank(&order.item, quantity, PostCraftAction::None);
                order.dec_in_progress(quantity);
                crafted.ok().map(|craft| craft.amount_of(&order.item))
            }
            Err(e) => {
                let CharacterError::InsuffisientMaterials = e else {
                    return None;
                };
                if !self.order_missing_mats(
                    &order.item,
                    self.orderboard.total_missing(order),
                    order.purpose.clone(),
                ) {
                    return None;
                }
                Some(0)
            }
        }
    }

    fn progress_task_reward_order(&self, order: &Order) -> Option<i32> {
        match self.can_exchange_task() {
            Ok(()) => {
                if order.in_progress() > 0 {
                    return None;
                }
                order.inc_in_progress(1);
                let exchanged = self.exchange_task().map(|r| r.amount_of(&order.item)).ok();
                order.dec_in_progress(1);
                exchanged
            }
            Err(e) => {
                if self.orderboard.total_missing(order) <= 0 {
                    return None;
                }
                if let CharacterError::NotEnoughCoin = e {
                    let q = 6 - if self.orderboard.is_ordered("tasks_coin") {
                        0
                    } else {
                        self.has_in_bank_or_inv("tasks_coin")
                    };
                    if q > 0
                        && self.orderboard.add(Order::new(
                            None,
                            "tasks_coin",
                            q,
                            order.purpose.to_owned(),
                        ))
                    {
                        return Some(0);
                    }
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
                    let r#type = if self.skill_enabled(Skill::Combat) {
                        TaskType::Monsters
                    } else {
                        TaskType::Items
                    };
                    if let Err(e) = self.accept_task(r#type) {
                        error!("{} error while accepting new task: {:?}", self.name, e)
                    }
                    return Some(0)
                }
                let CharacterError::TaskNotFinished = e else {
                    return None;
                };
                match self.progress_task() {
                    Ok(_) => Some(0),
                    Err(CharacterError::MissingItems { item, quantity }) => {
                        if self.orderboard.add(Order::new(
                            Some(&self.name),
                            &item,
                            quantity,
                            Purpose::Task {
                                char: self.name.to_owned(),
                            },
                        )) {
                            Some(0)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        }
    }

    /// Deposit items requiered by the given `order` if needed.
    /// Returns true if items has be deposited.
    fn turn_in_order(&self, order: Arc<Order>) -> bool {
        if self.orderboard.should_be_turned_in(&order) || self.inventory.is_full() {
            return self.deposit_order(&order);
        }
        false
    }

    fn deposit_order(&self, order: &Order) -> bool {
        let q = self.inventory.contains(&order.item);
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
            order.inc_deposited(q);
            if order.turned_in() {
                self.orderboard.remove(order);
            }
            return true;
        }
        false
    }

    fn progress_task(&self) -> Result<(), CharacterError> {
        if let Some(monster) = self.monsters.get(&self.task()) {
            if self.kill_monster(monster, None).is_ok() {
                return Ok(());
            }
        }
        self.trade_task()
    }

    fn trade_task(&self) -> Result<(), CharacterError> {
        if !self.task_type().is_some_and(|tt| tt == TaskType::Items) {
            return Err(CharacterError::InvalidTaskType);
        }
        if self.task_missing()
            > self.bank.has_item(&self.task(), Some(&self.name))
                + self.inventory.contains(&self.task())
        {
            return Err(CharacterError::MissingItems {
                item: self.task().to_owned(),
                quantity: self.task_missing()
                    - self.bank.has_item(&self.task(), Some(&self.name))
                    - self.inventory.contains(&self.task()),
            });
        }
        let q = min(self.task_missing(), self.inventory.max_items());
        if let Err(e) = self.bank.reserv_if_not(&self.task(), q, &self.name) {
            error!(
                "{}: error while reserving items for item task: {:?}",
                self.name, e
            )
        }
        self.deposit_all();
        if let Err(e) = self.withdraw_item(&self.task(), q) {
            error!("{}: error while withdrawing {:?}", self.name, e);
            self.bank.decrease_reservation(&self.task(), q, &self.name);
        };
        self.move_to_closest_taskmaster(self.task_type())?;
        self.action_task_trade(&self.task(), q)?;
        Ok(())
    }

    fn accept_task(&self, r#type: TaskType) -> Result<TaskSchema, CharacterError> {
        self.move_to_closest_taskmaster(Some(r#type))?;
        Ok(self.action_accept_task()?)
    }

    fn complete_task(&self) -> Result<TaskRewardsSchema, CharacterError> {
        if self.task().is_empty() {
            return Err(CharacterError::NoTask);
        }
        if !self.task_finished() {
            return Err(CharacterError::TaskNotFinished);
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        self.action_complete_task().map_err(|e| e.into())
    }

    fn can_exchange_task(&self) -> Result<(), CharacterError> {
        if self.bank.has_item("tasks_coin", Some(&self.name)) < 6 {
            return Err(CharacterError::NotEnoughCoin);
        }
        Ok(())
    }

    fn exchange_task(&self) -> Result<TaskRewardsSchema, CharacterError> {
        self.can_exchange_task()?;
        if self
            .bank
            .reserv_if_not("tasks_coin", 6, &self.name)
            .is_err()
        {
            return Err(CharacterError::NotEnoughCoin);
        }
        self.deposit_all();
        self.withdraw_item("tasks_coin", 6)?;
        if let Err(e) = self.inventory.reserv_items_if_not("tasks_coin", 6) {
            error!("{}: error while reserving 'tasks_coin': {:?}", self.name, e);
        }
        self.move_to_closest_taskmaster(self.task_type())?;
        let res = self.action_task_exchange().map_err(|e| e.into());
        self.inventory.decrease_reservation("tasks_coin", 6);
        res
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
        match self.complete_task() {
            Ok(_) | Err(CharacterError::NoTask) => {
                if let Err(e) = self.accept_task(TaskType::Monsters) {
                    error!("{} error while accepting new task: {:?}", self.name, e)
                }
            }
            _ => (),
        }
        if let Some(task_monster) = self.monsters.get(&self.task()) {
            if self.can_kill(task_monster).is_ok() {
                self.kill_monster(task_monster, None)?;
                return Ok(());
            }
        }
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
        self.withdraw_food();
        if let Ok(_) | Err(CharacterError::NoTask) = self.complete_task() {
            if let Err(e) = self.accept_task(TaskType::Monsters) {
                error!("{} error while accepting new task: {:?}", self.name, e)
            }
        }
        if let Some(map) = map {
            self.action_move(map.x, map.y)?;
        } else if let Some(map) = self.closest_map_with_content_code(&monster.code) {
            self.action_move(map.x, map.y)?;
        }
        if self
            .fight_simulator
            .simulate(self.level(), self.missing_hp(), &self.gear(), monster)
            .result
            == FightResult::Loss
        {
            self.eat_food();
        }
        if self
            .fight_simulator
            .simulate(self.level(), self.missing_hp(), &self.gear(), monster)
            .result
            == FightResult::Loss
        {
            if let Err(e) = self.rest() {
                error!("{} failed to rest: {:?}", self.name, e)
            }
        }
        Ok(self.action_fight()?)
    }

    fn rest(&self) -> Result<(), CharacterError> {
        if self.health() < self.max_health() {
            self.action_rest()?;
        }
        Ok(())
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
        let prev_equiped = self.equiped_in(Slot::Weapon);
        let tool = self.best_tool_for_resource(&resource.code);
        if let Some(prev_equiped) = prev_equiped {
            if tool.is_none() || tool.is_some_and(|t| t.code != prev_equiped.code) {
                if let Err(e) = self.action_unequip(Slot::Weapon, 1) {
                    error!(
                        "{}: failed to unequip previously equiped weapon: {:?}",
                        self.name, e
                    )
                }
                // TODO: improve logic: maybe include this logic in `deposit_item` method
                if let Some(o) = self
                    .orderboard
                    .orders_by_priority()
                    .iter()
                    .find(|o| o.item == prev_equiped.code)
                {
                    self.deposit_order(o);
                } else if let Err(e) = self.deposit_item(&prev_equiped.code, 1, None) {
                    error!(
                        "{}: error while depositing previously equiped weapon: {:?}",
                        self.name, e
                    )
                }
            }
        }
        let Some(tool) = tool else {
            return;
        };
        let Ok(_browsed) = self.bank.browsed.write() else {
            return;
        };
        if self.has_available(&tool.code) > 0 {
            self.reserv_if_needed_and_available(Slot::Weapon, tool, 1);
        } else {
            self.orderboard.add(Order::new(
                Some(&self.name),
                &tool.code,
                1,
                Purpose::Gather {
                    char: self.name.to_owned(),
                    skill: resource.skill.into(),
                    item_code: tool.code.to_owned(),
                },
            ));
        }
        drop(_browsed);
        if self.has_available(&tool.code) > 0 {
            self.equip_item_from_bank_or_inventory(Slot::Weapon, tool);
        }
    }

    fn time_to_kill(&self, monster: &MonsterSchema) -> Option<i32> {
        match self.can_kill(monster) {
            Ok(gear) => {
                let fight = self
                    .fight_simulator
                    .simulate(self.level(), 0, &gear, monster);
                if fight.result == FightResult::Win {
                    Some(fight.cd)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn time_to_gather(&self, resource: &ResourceSchema) -> Option<i32> {
        if self.can_gather(resource).is_err() {
            return None;
        }
        let tool = self.best_tool_for_resource(&resource.code);
        let time = self.fight_simulator.gather(
            self.skill_level(resource.skill.into()),
            resource.level,
            tool.map_or(0, |t| t.skill_cooldown_reduction(resource.skill.into())),
        );
        Some(time)
    }

    #[allow(dead_code)]
    fn time_to_get(&self, item: &str) -> Option<i32> {
        self.items
            .sources_of(item)
            .iter()
            .filter_map(|s| match s {
                ItemSource::Resource(r) => self.time_to_gather(r),
                ItemSource::Monster(m) => self
                    .time_to_kill(m)
                    .map(|time| time * self.items.drop_rate(item)),
                ItemSource::Craft => Some(
                    self.items
                        .mats(item)
                        .iter()
                        .map(|m| self.time_to_get(&m.code).unwrap_or(1000) * m.quantity)
                        .sum(),
                ),
                ItemSource::TaskReward => Some(1500),
                ItemSource::Task => Some(1500),
            })
            .min()
    }

    /// Checks if the `Character` is able to kill the given monster and returns
    /// the best available gear to do so.
    fn can_kill<'a>(&'a self, monster: &'a MonsterSchema) -> Result<Gear<'_>, CharacterError> {
        if !self.skill_enabled(Skill::Combat) {
            return Err(CharacterError::SkillDisabled);
        }
        if self.inventory.is_full() {
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
            .simulate(self.level(), 0, gear, monster)
            .result
            == FightResult::Win
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
        if self.inventory.is_full() {
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
        if self.inventory.is_full() {
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
            utility1: self.items.get(&d.utility1_slot),
            utility2: self.items.get(&d.utility2_slot),
        }
    }

    /// Returns the item equiped in the `given` slot.
    fn equiped_in(&self, slot: Slot) -> Option<&ItemSchema> {
        let d = self.data.read().unwrap();
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
            Slot::Utility1 => &d.utility1_slot,
            Slot::Utility2 => &d.utility2_slot,
        })
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
                .map_or(0, |s| s.amount_of(code));
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
    ) -> Result<SkillInfoSchema, CharacterError> {
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
    ) -> Result<SkillInfoSchema, CharacterError> {
        self.can_craft(code)?;
        self.craft_from_bank_unchecked(code, quantity, post_action)
    }

    pub fn craft_from_bank_unchecked(
        &self,
        code: &str,
        quantity: i32,
        post_action: PostCraftAction,
    ) -> Result<SkillInfoSchema, CharacterError> {
        if self.max_craftable_items_from_bank(code) < quantity {
            return Err(CharacterError::InsuffisientMaterials);
        }
        info!(
            "{}: going to craft '{}'x{} from bank.",
            self.name, code, quantity
        );
        self.items.mats(code).iter().for_each(|m| {
            if let Err(e) = self
                .bank
                .reserv_if_not(&m.code, m.quantity * quantity, &self.name)
            {
                error!(
                    "{}: error while reserving mats for crafting from bank: {:?}",
                    self.name, e
                )
            }
        });
        self.deposit_all();
        let mats = self.withdraw_mats_for(code, quantity)?;
        mats.iter().for_each(|m| {
            if let Err(e) = self.inventory.reserv_items_if_not(&m.code, m.quantity) {
                error!(
                    "{}: error while reserving mats for crafting from inventory: {:?}",
                    self.name, e
                )
            }
        });
        self.move_to_craft(code)?;
        let craft = self.action_craft(code, quantity)?;
        mats.iter().for_each(|m| {
            self.inventory.decrease_reservation(&m.code, m.quantity);
        });
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
        Ok(craft)
    }

    pub fn recycle_from_bank(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<RecyclingItemsSchema, CharacterError> {
        self.can_craft(code)?;
        if self.bank.has_item(code, Some(&self.name)) < quantity {
            return Err(CharacterError::ItemNotFound);
        }
        info!("{}: going to recycle '{}x{}'.", self.name, code, quantity);
        if let Err(e) = self.bank.reserv_if_not(code, quantity, &self.name) {
            error!("{}: error while reserving '{}': {:?}", self.name, code, e);
        }
        self.deposit_all();
        self.withdraw_item(code, quantity)?;
        self.move_to_craft(code)?;
        Ok(self.action_recycle(code, quantity)?)
    }

    pub fn deposit_item(
        &self,
        item: &str,
        quantity: i32,
        owner: Option<String>,
    ) -> Result<SimpleItemSchema, CharacterError> {
        if self.inventory.contains(item) < quantity {
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
            if let Some(res) = self.inventory.get_reservation(item) {
                res.dec_quantity(quantity);
                if res.quantity() <= 0 {
                    self.inventory.remove_reservation(&res);
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
        if self.inventory.total() <= 0 {
            return;
        }
        info!("{}: going to deposit all items to the bank.", self.name,);
        self.orderboard.orders_by_priority().iter().for_each(|o| {
            self.deposit_order(o);
        });
        self.inventory.copy().iter().for_each(|slot| {
            if slot.quantity > 0 {
                if let Err(e) = self.deposit_item(&slot.code, slot.quantity, None) {
                    error!("{}: error while depositing all to bank: {:?}", self.name, e)
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
                let quantity = min(self.inventory.free_space(), remain);
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

    /// Withdraw the materials required to craft the `quantity` of the
    /// item `code` and returns the maximum amount that can be crafted.
    // TODO: add check on `inventory_max_items`
    fn withdraw_mats_for(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<Vec<SimpleItemSchema>, CharacterError> {
        let mats = self
            .items
            .mats(code)
            .into_iter()
            .update(|m| m.quantity *= quantity)
            .collect_vec();
        for mat in &mats {
            if self.bank.has_item(&mat.code, Some(&self.name)) < mat.quantity {
                warn!("{}: not enough materials in bank to withdraw the materials required to craft '{code}'x{quantity}", self.name);
                return Err(CharacterError::InsuffisientMaterials);
            }
        }
        info!(
            "{}: going to withdraw materials for '{code}'x{quantity}.",
            self.name
        );
        for mat in &mats {
            self.withdraw_item(&mat.code, mat.quantity)?;
        }
        Ok(mats)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on
    /// inventory max items
    fn max_craftable_items(&self, code: &str) -> i32 {
        self.inventory.max_items() / self.items.mats_quantity_for(code)
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory max items and bank materials.
    fn max_craftable_items_from_bank(&self, code: &str) -> i32 {
        min(
            self.bank.has_mats_for(code, Some(&self.name)),
            self.inventory.max_items() / self.items.mats_quantity_for(code),
        )
    }

    /// Calculates the maximum number of items that can be crafted in one go based on available
    /// inventory free space and bank materials.
    fn max_current_craftable_items(&self, code: &str) -> i32 {
        min(
            self.bank.has_mats_for(code, Some(&self.name)),
            self.inventory.free_space() / self.items.mats_quantity_for(code),
        )
    }

    /// Reycle the maximum amount of the item `code` with the items  currently
    /// available in the character inventory and returns the amount recycled.
    pub fn recycle_all(&self, code: &str) -> i32 {
        let n = self.inventory.contains(code);
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

    fn food_in_inventory(&self) -> Vec<&ItemSchema> {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|&i| {
                    i.is_of_type(Type::Consumable)
                        && i.heal() > 0
                        && i.level <= self.level()
                        && i.code != "apple"
                        && i.code != "egg"
                })
            })
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

    fn move_to_closest_taskmaster(
        &self,
        r#type: Option<TaskType>,
    ) -> Result<MapSchema, CharacterError> {
        if let Some(r#type) = r#type {
            self.move_to_closest_map_with_content_schema(&MapContentSchema {
                r#type: "tasks_master".to_owned(),
                code: r#type.to_string(),
            })
        } else {
            self.move_to_closest_map_of_type("tasks_master")
        }
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

    fn equip_item(&self, item: &str, slot: Slot, quantity: i32) -> Result<(), CharacterError> {
        // TODO: add check from inventory space
        if let Some(equiped) = self.equiped_in(slot) {
            if equiped.health() >= self.health() {
                // TODO: eat food if possible
                self.rest()?;
            }
            self.action_unequip(slot, self.quantity_in_slot(slot))?;
        }
        self.action_equip(item, slot, quantity)?;
        Ok(())
    }

    fn equip_item_from_bank_or_inventory(&self, slot: Slot, item: &ItemSchema) {
        let prev_equiped = self.equiped_in(slot);
        if prev_equiped.is_some_and(|e| e.code == item.code) {
            return;
        }
        if self.inventory.contains(&item.code) <= 0
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
        if let Err(e) = self.equip_item(&item.code, slot, 1) {
            error!(
                "{} failed to equip item from bank or inventory: {:?}",
                self.name, e
            );
        }
        if let Some(i) = prev_equiped {
            // TODO: improve logic
            if let Some(o) = self
                .orderboard
                .orders_by_priority()
                .iter()
                .find(|o| o.item == i.code)
            {
                self.deposit_order(o);
            } else if let Err(e) = self.deposit_item(&i.code, 1, None) {
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
        self.bank.has_item(code, Some(&self.name)) + self.inventory.contains(code)
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

    fn max_health(&self) -> i32 {
        self.data.read().unwrap().max_hp
    }

    fn health(&self) -> i32 {
        self.data.read().unwrap().hp
    }

    fn missing_hp(&self) -> i32 {
        self.data.read().unwrap().max_hp - self.data.read().unwrap().hp
    }

    fn task(&self) -> String {
        self.data.read().unwrap().task.to_owned()
    }

    fn task_type(&self) -> Option<TaskType> {
        match self.data.read().unwrap().task_type.as_str() {
            "monsters" => Some(TaskType::Monsters),
            "items" => Some(TaskType::Items),
            _ => None,
        }
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
    pub fn skill_level(&self, skill: Skill) -> i32 {
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
            Skill::Alchemy => d.alchemy_level,
        }
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }

    #[allow(dead_code)]
    fn time_to_get_gear(&self, gear: &Gear) -> Option<i32> {
        Slot::iter()
            .map(|s| gear.slot(s).and_then(|i| self.items.time_to_get(&i.code)))
            .sum()
    }

    //TODO: finish implementing this function
    #[allow(dead_code)]
    #[allow(unused_variables)]
    fn order_upgrades(&self, current: Gear<'_>, monster: &MonsterSchema, filter: Filter) {
        let gears = self.gear_finder.bests_against(self, monster, filter);
        if let Some(gear) = gears
            .iter()
            .filter(|g| self.can_kill_with(monster, g))
            .min_by_key(|g| self.time_to_get_gear(g))
        {
            if self.can_kill_with(monster, gear) {
                self.order_gear(*gear);
            };
        }
    }

    fn order_best_gear_against(&self, monster: &MonsterSchema, filter: Filter) {
        let gear = self.gear_finder.best_against(self, monster, filter);
        if self.can_kill_with(monster, &gear) {
            self.order_gear(gear);
        };
    }

    fn order_gear(&self, gear: Gear<'_>) {
        Slot::iter().for_each(|s| {
            if !s.is_ring_1() && !s.is_ring_2() {
                if let Some(item) = gear.slot(s) {
                    let quantity = if s.is_utility_1() || s.is_utility_2() {
                        100
                    } else {
                        1
                    };
                    self.order_if_needed(s, item, quantity);
                }
            }
        });
        if gear.ring1.is_some() && gear.ring1 == gear.ring2 {
            self.order_if_needed(Slot::Ring1, gear.ring1.unwrap(), 2);
        } else {
            if let Some(ring1) = gear.ring1 {
                self.order_if_needed(Slot::Ring1, ring1, 1);
            }
            if let Some(ring2) = gear.ring1 {
                self.order_if_needed(Slot::Ring2, ring2, 1);
            }
        }
    }

    fn order_if_needed(&self, slot: Slot, item: &ItemSchema, quantity: i32) -> bool {
        if (self.equiped_in(slot).is_none()
            || self
                .equiped_in(slot)
                .is_some_and(|equiped| item.code != equiped.code))
            && self.has_in_bank_or_inv(&item.code) < quantity
        {
            let quantity = quantity - self.has_available(&item.code);
            if quantity > 0 {
                self.orderboard.add(Order::new(
                    None,
                    &item.code,
                    quantity,
                    Purpose::Gear {
                        char: self.name.to_owned(),
                        slot,
                        item_code: item.code.to_owned(),
                    },
                ));
                return true;
            }
        }
        false
    }

    fn reserv_gear(&self, gear: Gear<'_>) {
        Slot::iter().for_each(|s| {
            if !(s.is_ring_1() || s.is_ring_2()) {
                if let Some(item) = gear.slot(s) {
                    let quantity = if s.is_utility_1() || s.is_utility_2() {
                        100
                    } else {
                        1
                    };
                    self.reserv_if_needed_and_available(s, item, quantity);
                }
            }
        });
        if gear.ring1.is_some() && gear.ring1 == gear.ring2 {
            self.reserv_if_needed_and_available(Slot::Ring1, gear.ring1.unwrap(), 2);
        } else {
            if let Some(ring1) = gear.ring1 {
                self.reserv_if_needed_and_available(Slot::Ring1, ring1, 1);
            }
            if let Some(ring2) = gear.ring2 {
                self.reserv_if_needed_and_available(Slot::Ring2, ring2, 1);
            }
        }
    }

    /// Reserves the given `quantity` of the `item` if needed and available.
    fn reserv_if_needed_and_available(&self, s: Slot, item: &ItemSchema, quantity: i32) {
        if (self.equiped_in(s).is_none()
            || self
                .equiped_in(s)
                .is_some_and(|equiped| item.code != equiped.code))
            && self.inventory.contains(&item.code) < quantity
        {
            if let Err(e) = self.bank.reserv_if_not(
                &item.code,
                quantity - self.inventory.contains(&item.code),
                &self.name,
            ) {
                error!("{} failed to reserv '{}': {:?}", self.name, item.code, e)
            }
        }
    }

    pub fn unequip_and_deposit_all(&self) {
        Slot::iter().for_each(|s| {
            if let Some(item) = self.equiped_in(s) {
                let quantity = self.quantity_in_slot(s);
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

    fn quantity_in_slot(&self, s: Slot) -> i32 {
        match s {
            Slot::Utility1 => self.data.read().unwrap().utility1_slot_quantity,
            Slot::Utility2 => self.data.read().unwrap().utility2_slot_quantity,
            Slot::Weapon
            | Slot::Shield
            | Slot::Helmet
            | Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Boots
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet
            | Slot::Artifact1
            | Slot::Artifact2
            | Slot::Artifact3 => 1,
        }
    }

    fn skill_enabled(&self, s: Skill) -> bool {
        self.conf().skills.contains(&s)
    }

    fn withdraw_food(&self) {
        let Ok(_browsed) = self.bank.browsed.write() else {
            return;
        };
        self.order_food();
        if !self.food_in_inventory().is_empty() {
            return;
        }
        let Some(food) = self
            .bank
            .food()
            .into_iter()
            .filter(|f| {
                f.level <= self.level() && self.bank.has_item(&f.code, Some(&self.name)) > 0
            })
            .max_by_key(|f| f.heal())
        else {
            return;
        };
        let quantity = min(
            self.inventory.max_items() - 30,
            self.bank.has_item(&food.code, Some(&self.name)),
        );
        if let Err(e) = self.bank.reserv_if_not(&food.code, quantity, &self.name) {
            error!("{} failed to reserv food: {:?}", self.name, e)
        };
        drop(_browsed);
        self.deposit_all();
        if let Err(e) = self.withdraw_item(&food.code, quantity) {
            error!("{} failed to withdraw food: {:?}", self.name, e)
        }
        if let Err(e) = self.inventory.reserv_items_if_not(&food.code, quantity) {
            error!("{} failed to reserv food: {:?}", self.name, e)
        };
    }

    fn order_food(&self) {
        if let Some(best_food) = self
            .items
            .consumable_food(self.level())
            .iter()
            .min_by_key(|i| {
                self.bank
                    .missing_mats_quantity(&i.code, self.inventory.max_items() - 30, None)
            })
        {
            let quantity = 500 - self.bank.has_item(&best_food.code, Some(&self.name));
            if quantity > 0 {
                self.orderboard.add_or_reset(Order::new(
                    Some(&self.name),
                    &best_food.code,
                    quantity,
                    Purpose::Food {
                        char: self.name.to_owned(),
                    },
                ));
            }
        }
    }

    fn eat_food(&self) {
        self.food_in_inventory()
            .iter()
            .sorted_by_key(|i| i.heal())
            .for_each(|f| {
                // TODO: improve logic to eat different foods to restore more hp
                // also add a threasholf to allow some overheal
                let quantity = min(
                    self.missing_hp() / f.heal(),
                    self.inventory.contains(&f.code),
                );
                if quantity > 0 {
                    if let Err(e) = self.action_use_item(&f.code, quantity) {
                        error!("{} failed to use food: {:?}", self.name, e)
                    }
                    if let Some(res) = self.inventory.get_reservation(&f.code) {
                        res.dec_quantity(quantity);
                        if res.quantity() <= 0 {
                            self.inventory.remove_reservation(&res);
                        }
                    }
                }
            });
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
    InvalidTaskType,
    MissingItems { item: String, quantity: i32 },
    QuantityUnavailable(i32),
}

impl From<RequestError> for CharacterError {
    fn from(value: RequestError) -> Self {
        CharacterError::RequestError(value)
    }
}
