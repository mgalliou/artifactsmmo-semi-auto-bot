use crate::{
    AccountClient, Code, CollectionClient, GOLD, Gear, HasConditions, ItemContainer, Level,
    LimitedContainer, Quantity, SlotLimited, SpaceLimited, TASK_EXCHANGE_PRICE, TASKS_COIN,
    TasksClient,
    character::error::{
        ClaimPendingItemError, GeBuyOrderError, GeCancelOrderError, GeCreateOrderError,
        GiveGoldError, GiveItemError, TransitionError,
    },
    client::{
        bank::{Bank, BankClient},
        character::error::{
            BankExpansionError, BuyNpcError, CraftError, DeleteError, DepositError, EquipError,
            FightError, GatherError, GoldDepositError, GoldWithdrawError, MoveError, RecycleError,
            RestError, SellNpcError, TaskAcceptationError, TaskCancellationError,
            TaskCompletionError, TaskTradeError, TasksCoinExchangeError, UnequipError, UseError,
            WithdrawError,
        },
        items::{ItemsClient, LevelConditionCode},
        maps::MapsClient,
        monsters::MonstersClient,
        npcs::NpcsClient,
        resources::ResourcesClient,
    },
    entities::{AccountAchievement, Character, CharacterHandle, CharacterName, Map, RawMap},
    gear::Slot,
    grand_exchange::GrandExchangeClient,
    simulator::HasEffects,
    skill::Skill,
};
use derive_more::Deref;
use openapi::models::{
    CharacterFightSchema, ConditionOperator, EquipSchema, GeOrderType, GeTransactionSchema,
    MapContentType, NpcItemTransactionSchema, RecyclingItemsSchema, RewardsSchema,
    SimpleItemSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, TaskType, UnequipSchema,
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

pub use handler::CharacterRequestHandler;
pub use inventory::{Inventory, InventoryClient};

pub mod handler;
pub(crate) mod request_handler;

pub mod action_request;
pub mod error;
pub mod inventory;
pub mod responses;

/// Client for handling the actions and accessing the state of an
/// `ArtifactsMMO` character.
///
/// Delegates API calls through a pluggable request handler. Every action
/// follows a **validate-then-execute** pattern: a `can_*` method checks all
/// preconditions locally and returns a typed error, so invalid requests are
/// caught early without hitting the API, reducing latency and usage.
#[derive(Clone, Deref)]
#[deref(forward)]
pub struct CharacterClient(Arc<CharacterClientInner>);

#[derive(Deref)]
pub struct CharacterClientInner {
    pub id: usize,
    #[deref]
    data: CharacterHandle,
    handler: Arc<dyn CharacterRequestHandler>,
    inventory: InventoryClient,
    account: AccountClient,
    bank: BankClient,
    items: ItemsClient,
    resources: ResourcesClient,
    monsters: MonstersClient,
    maps: MapsClient,
    npcs: NpcsClient,
    tasks: TasksClient,
    grand_exchange: GrandExchangeClient,
    gear_cache: Mutex<Option<Gear>>,
}

impl CharacterClient {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) fn new(
        id: usize,
        data: CharacterHandle,
        handler: Arc<dyn CharacterRequestHandler>,
        account: AccountClient,
        items: ItemsClient,
        resources: ResourcesClient,
        monsters: MonstersClient,
        maps: MapsClient,
        npcs: NpcsClient,
        tasks: TasksClient,
        grand_exchange: GrandExchangeClient,
    ) -> Self {
        Self(Arc::new(CharacterClientInner {
            id,
            data: data.clone(),
            handler,
            inventory: InventoryClient::new(data),
            bank: account.bank(),
            account,
            items,
            resources,
            monsters,
            maps,
            npcs,
            tasks,
            grand_exchange,
            gear_cache: Mutex::new(None),
        }))
    }

    #[must_use]
    pub fn id(&self) -> usize {
        self.id
    }

    #[must_use]
    pub fn handler(&self) -> &dyn CharacterRequestHandler {
        &*self.handler
    }

    pub fn pause(&self) {
        self.handler().pause();
    }

    pub fn resume(&self) {
        self.handler().resume();
    }

    pub fn cancel(&self) {
        self.handler().cancel();
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.handler().is_paused()
    }

    #[must_use]
    pub fn inventory(&self) -> &InventoryClient {
        &self.inventory
    }

    pub fn r#move(&self, x: i32, y: i32) -> Result<RawMap, MoveError> {
        self.can_move(x, y)?;
        Ok(self.handler().request_move(x, y)?)
    }

    pub fn can_move(&self, x: i32, y: i32) -> Result<(), MoveError> {
        let position = self.position();
        let destination = (position.0, x, y);
        if position == destination {
            return Err(MoveError::AlreadyOnMap);
        }
        let Some(map) = self.maps.get_raw(&destination) else {
            return Err(MoveError::MapNotFound);
        };
        if map.is_blocked() || !self.meets_conditions_for(map.access()) {
            return Err(MoveError::ConditionsNotMet);
        }
        // TODO: check map is accesible
        Ok(())
    }

    pub fn transition(&self) -> Result<RawMap, TransitionError> {
        self.can_transition()?;
        Ok(self.handler().request_transition()?)
    }

    pub fn can_transition(&self) -> Result<(), TransitionError> {
        let map = self.current_map();
        let Some(transition) = map.transition() else {
            return Err(TransitionError::TransitionNotFound);
        };
        if !self.meets_conditions_for(transition) {
            return Err(TransitionError::ConditionsNotMet);
        }
        Ok(())
    }

    pub fn fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, FightError> {
        self.can_fight(participants)?;
        Ok(self.handler().request_fight(participants)?)
    }

    pub fn can_fight(&self, participants: Option<&[String; 2]>) -> Result<(), FightError> {
        let Some(monster) = self
            .current_map()
            .content_code()
            .and_then(|code| self.monsters.get(code))
        else {
            return Err(FightError::NoMonsterOnMap);
        };
        if !self.inventory().has_room_for_drops_from(&monster) {
            return Err(FightError::InsufficientInventorySpace);
        }
        let Some(participants) = participants else {
            return Ok(());
        };
        if participants.is_empty() {
            return Ok(());
        }
        if !monster.is_boss() {
            return Err(FightError::MonsterIsNotABoss);
        }
        for name in participants {
            let Some(char) = self.account.get_character(name) else {
                return Err(FightError::CharacterNotFound);
            };
            if char.position() != self.position() {
                return Err(FightError::NoMonsterOnMap);
            }
            if !char.inventory().has_room_for_drops_from(&monster) {
                return Err(FightError::InsufficientInventorySpace);
            }
        }
        Ok(())
    }

    pub fn gather(&self) -> Result<SkillInfoSchema, GatherError> {
        self.can_gather()?;
        Ok(self.handler().request_gather()?)
    }

    pub fn can_gather(&self) -> Result<(), GatherError> {
        let Some(resource) = self
            .current_map()
            .content_code()
            .and_then(|code| self.resources.get(code))
        else {
            return Err(GatherError::NoResourceOnMap);
        };
        if self.skill_level(resource.skill()) < resource.level() {
            return Err(GatherError::SkillLevelInsufficient);
        }
        if !self.inventory().has_room_for_drops_from(&resource) {
            return Err(GatherError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn rest(&self) -> Result<u32, RestError> {
        if self.hp() < self.max_hp() {
            return Ok(self.handler().request_rest()?);
        }
        Ok(0)
    }

    pub fn craft(&self, item_code: &str, quantity: u32) -> Result<SkillInfoSchema, CraftError> {
        self.can_craft(item_code, quantity)?;
        Ok(self.handler().request_craft(item_code, quantity)?)
    }

    pub fn can_craft(&self, item_code: &str, quantity: u32) -> Result<(), CraftError> {
        let Some(item) = self.items.get(item_code) else {
            return Err(CraftError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CraftError::ItemNotCraftable);
        };
        if self.skill_level(skill) < item.level() {
            return Err(CraftError::InsufficientSkillLevel);
        }
        if !self.inventory().contains_all(&item.mats_for(quantity)) {
            return Err(CraftError::InsufficientMaterials);
        }
        if !self.inventory().has_room_to_craft(&item) {
            return Err(CraftError::InsufficientInventorySpace);
        }
        if !self.current_map().content_code_is(skill.as_ref()) {
            return Err(CraftError::NoWorkshopOnMap);
        }
        Ok(())
    }

    pub fn recycle(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<RecyclingItemsSchema, RecycleError> {
        self.can_recycle(item_code, quantity)?;
        Ok(self.handler().request_recycle(item_code, quantity)?)
    }

    pub fn can_recycle(&self, item_code: &str, quantity: u32) -> Result<(), RecycleError> {
        let Some(item) = self.items.get(item_code) else {
            return Err(RecycleError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(RecycleError::ItemNotRecyclable);
        };
        if skill.is_cooking() || skill.is_alchemy() {
            return Err(RecycleError::ItemNotRecyclable);
        }
        if self.skill_level(skill) < item.level() {
            return Err(RecycleError::InsufficientSkillLevel);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(RecycleError::InsufficientQuantity);
        }
        if self.inventory().free_space() + quantity < item.recycled_quantity() {
            return Err(RecycleError::InsufficientInventorySpace);
        }
        if !self.current_map().content_code_is(skill.as_ref()) {
            return Err(RecycleError::NoWorkshopOnMap);
        }
        Ok(())
    }

    pub fn delete(&self, item_code: &str, quantity: u32) -> Result<SimpleItemSchema, DeleteError> {
        self.can_delete(item_code, quantity)?;
        Ok(self.handler().request_delete(item_code, quantity)?)
    }

    pub fn can_delete(&self, item_code: &str, quantity: u32) -> Result<(), DeleteError> {
        if self.items.get(item_code).is_none() {
            return Err(DeleteError::ItemNotFound);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(DeleteError::InsufficientQuantity);
        }
        Ok(())
    }

    pub fn deposit_item(&self, items: &[SimpleItemSchema]) -> Result<(), DepositError> {
        self.can_deposit_items(items)?;
        Ok(self.handler().request_deposit_item(items)?)
    }

    pub fn can_deposit_items(&self, items: &[SimpleItemSchema]) -> Result<(), DepositError> {
        for item in items {
            if self.items.get(&item.code).is_none() {
                return Err(DepositError::ItemNotFound);
            }
            if self.inventory().total_of(&item.code) < item.quantity() {
                return Err(DepositError::InsufficientQuantity);
            }
        }
        if !self.bank.has_room_for_all(items) {
            return Err(DepositError::InsufficientBankSpace);
        }
        if !self.current_map().is_bank() {
            return Err(DepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), WithdrawError> {
        self.can_withdraw_items(items)?;
        Ok(self.handler().request_withdraw_item(items)?)
    }

    pub fn can_withdraw_items(&self, items: &[SimpleItemSchema]) -> Result<(), WithdrawError> {
        if items
            .iter()
            .any(|i| self.bank.total_of(&i.code) < i.quantity())
        {
            return Err(WithdrawError::InsufficientQuantity);
        }
        if !self.inventory().has_room_for_all(items) {
            return Err(WithdrawError::InsufficientInventorySpace);
        }
        if !self.current_map().is_bank() {
            return Err(WithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn deposit_gold(&self, quantity: u32) -> Result<u32, GoldDepositError> {
        self.can_deposit_gold(quantity)?;
        Ok(self.handler().request_deposit_gold(quantity)?)
    }

    pub fn can_deposit_gold(&self, quantity: u32) -> Result<(), GoldDepositError> {
        if self.gold() < quantity {
            return Err(GoldDepositError::InsufficientGold);
        }
        if !self.current_map().is_bank() {
            return Err(GoldDepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn withdraw_gold(&self, quantity: u32) -> Result<u32, GoldWithdrawError> {
        self.can_withdraw_gold(quantity)?;
        Ok(self.handler().request_withdraw_gold(quantity)?)
    }

    pub fn can_withdraw_gold(&self, quantity: u32) -> Result<(), GoldWithdrawError> {
        if self.bank.gold() < quantity {
            return Err(GoldWithdrawError::InsufficientGold);
        }
        if !self.current_map().is_bank() {
            return Err(GoldWithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn expand_bank(&self) -> Result<u32, BankExpansionError> {
        self.can_expand_bank()?;
        Ok(self.handler().request_expand_bank()?)
    }

    pub fn can_expand_bank(&self) -> Result<(), BankExpansionError> {
        if self.gold() < self.bank.next_expansion_cost() {
            return Err(BankExpansionError::InsufficientGold);
        }
        if !self.current_map().is_bank() {
            return Err(BankExpansionError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn equip(&self, items: &[EquipSchema]) -> Result<(), EquipError> {
        self.can_equip(items)?;
        self.handler().request_equip(items)?;
        self.invalidate_gear_cache();
        Ok(())
    }

    pub fn can_equip(&self, items: &[EquipSchema]) -> Result<(), EquipError> {
        let mut total_quantity = 0;
        let mut inventory_space = 0;

        for schema in items {
            let Some(item) = self.items.get(&schema.code) else {
                return Err(EquipError::ItemNotFound);
            };
            let quantity = schema.quantity.unwrap_or(1);
            let slot = Slot::from(schema.slot);

            total_quantity += quantity;
            inventory_space += item.inventory_space();
            if self.inventory().total_of(item.code()) < quantity {
                return Err(EquipError::InsufficientQuantity);
            } else if !self.meets_conditions_for(&item) {
                return Err(EquipError::ConditionsNotMet);
            }
            let Some(equiped) = self.items.get(&self.equiped_in(slot)) else {
                continue;
            };
            if equiped.code() != item.code() {
                return Err(EquipError::SlotNotEmpty);
            } else if slot.max_quantity() <= 1 {
                return Err(EquipError::ItemAlreadyEquiped);
            } else if self.quantity_in_slot(slot) + quantity > slot.max_quantity() {
                return Err(EquipError::InsufficientSlotSpace);
            }
        }
        if self.inventory().free_space() as i32 + total_quantity as i32 + inventory_space <= 0 {
            return Err(EquipError::InsufficientInventorySpace);
        }

        Ok(())
    }

    pub fn unequip(&self, slots: &[UnequipSchema]) -> Result<(), UnequipError> {
        self.can_unequip(slots)?;
        self.handler().request_unequip(slots)?;
        self.invalidate_gear_cache();
        Ok(())
    }

    pub fn can_unequip(&self, slots: &[UnequipSchema]) -> Result<(), UnequipError> {
        let mut total_quantity = 0;
        let mut health = 0;
        let mut inventory_space = 0;
        let mut items: Vec<SimpleItemSchema> = vec![];

        for schema in slots {
            let slot = Slot::from(schema.slot);
            let Some(equiped) = self.items.get(&self.equiped_in(slot)) else {
                return Err(UnequipError::SlotEmpty);
            };
            let quantity_in_slot = self.quantity_in_slot(slot);
            let quantity = schema.quantity.unwrap_or(quantity_in_slot);

            health += equiped.health();
            if quantity_in_slot < quantity {
                return Err(UnequipError::InsufficientQuantity);
            }
            total_quantity += quantity;
            inventory_space += equiped.inventory_space();
            items.push(SimpleItemSchema {
                code: equiped.code().to_owned(),
                quantity,
            });
        }
        if self.hp() <= health {
            return Err(UnequipError::InsufficientHealth);
        } else if !self.inventory().has_room_for_all(&items)
            || self.inventory().free_space() as i32 - total_quantity as i32 - inventory_space <= 0
        {
            return Err(UnequipError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn use_item(&self, item_code: &str, quantity: u32) -> Result<(), UseError> {
        self.can_use_item(item_code, quantity)?;
        Ok(self.handler().request_use_item(item_code, quantity)?)
    }

    pub fn can_use_item(&self, item_code: &str, quantity: u32) -> Result<(), UseError> {
        let Some(item) = self.items.get(item_code) else {
            return Err(UseError::ItemNotFound);
        };
        if !item.is_consumable() {
            return Err(UseError::ItemNotConsumable);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(UseError::InsufficientQuantity);
        }
        if self.level() < item.level() {
            return Err(UseError::InsufficientCharacterLevel);
        }
        Ok(())
    }

    pub fn accept_task(&self) -> Result<TaskSchema, TaskAcceptationError> {
        self.can_accept_task()?;
        Ok(self.handler().request_accept_task()?)
    }

    pub fn can_accept_task(&self) -> Result<(), TaskAcceptationError> {
        if !self.task().is_empty() {
            return Err(TaskAcceptationError::TaskAlreadyInProgress);
        }
        if !self
            .current_map()
            .content_type_is(MapContentType::TasksMaster)
        {
            return Err(TaskAcceptationError::NoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn cancel_task(&self) -> Result<(), TaskCancellationError> {
        self.can_cancel_task()?;
        Ok(self.handler().request_cancel_task()?)
    }

    pub fn can_cancel_task(&self) -> Result<(), TaskCancellationError> {
        let Some(task_type) = self.task_type() else {
            return Err(TaskCancellationError::NoCurrentTask);
        };
        if self.inventory().total_of(TASKS_COIN) < 1 {
            return Err(TaskCancellationError::InsufficientTasksCoinQuantity);
        }
        if !self
            .current_map()
            .content_type_is(MapContentType::TasksMaster)
            || !self.current_map().content_code_is(&task_type.to_string())
        {
            return Err(TaskCancellationError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<TaskTradeSchema, TaskTradeError> {
        self.can_trade_task_item(item_code, quantity)?;
        Ok(self
            .handler()
            .request_trade_task_item(item_code, quantity)?)
    }

    pub fn can_trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<(), TaskTradeError> {
        if self.items.get(item_code).is_none() {
            return Err(TaskTradeError::ItemNotFound);
        }
        if *item_code != *self.task() {
            return Err(TaskTradeError::WrongTask);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(TaskTradeError::InsufficientQuantity);
        }
        if self.task_missing() < quantity {
            return Err(TaskTradeError::SuperfluousQuantity);
        }
        if !self.current_map().is_tasksmaster(TaskType::Items) {
            return Err(TaskTradeError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn complete_task(&self) -> Result<RewardsSchema, TaskCompletionError> {
        self.can_complete_task()?;
        Ok(self.handler().request_complete_task()?)
    }

    pub fn can_complete_task(&self) -> Result<(), TaskCompletionError> {
        let Some(task) = self.tasks.get::<str>(&self.task()) else {
            return Err(TaskCompletionError::NoCurrentTask);
        };
        if !self.task_finished() {
            return Err(TaskCompletionError::TaskNotFullfilled);
        }
        if !self.inventory().has_room_for_all(&task.rewards().items) {
            return Err(TaskCompletionError::InsufficientInventorySpace);
        }
        if !self.current_map().is_tasksmaster(task.r#type()) {
            return Err(TaskCompletionError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn exchange_tasks_coins(&self) -> Result<RewardsSchema, TasksCoinExchangeError> {
        self.can_exchange_tasks_coins()?;
        Ok(self.handler().request_exchange_tasks_coin()?)
    }

    pub fn can_exchange_tasks_coins(&self) -> Result<(), TasksCoinExchangeError> {
        let coins_in_inv = self.inventory().total_of(TASKS_COIN);
        if coins_in_inv < TASK_EXCHANGE_PRICE {
            return Err(TasksCoinExchangeError::InsufficientTasksCoinQuantity);
        }
        let extra_quantity = self
            .tasks
            .rewards()
            .max_quantity()
            .saturating_sub(TASK_EXCHANGE_PRICE);
        if self.inventory().free_space() < extra_quantity
            || self.inventory().free_slots() < 1 && coins_in_inv > TASK_EXCHANGE_PRICE
        {
            return Err(TasksCoinExchangeError::InsufficientInventorySpace);
        }
        if !self.current_map().is_tasksmaster(None) {
            return Err(TasksCoinExchangeError::NoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn npc_buy(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, BuyNpcError> {
        self.can_npc_buy(item_code, quantity)?;
        Ok(self.handler().request_npc_buy(item_code, quantity)?)
    }

    fn can_npc_buy(&self, item_code: &str, quantity: u32) -> Result<(), BuyNpcError> {
        if self.items.get(item_code).is_none() {
            return Err(BuyNpcError::ItemNotFound);
        }
        let Some(item) = self.npcs.items().get(item_code) else {
            return Err(BuyNpcError::ItemNotBuyable);
        };
        let Some(buy_price) = item.buy_price() else {
            return Err(BuyNpcError::ItemNotBuyable);
        };
        let total_price = buy_price * quantity;
        if item.currency() == GOLD && self.gold() < total_price {
            return Err(BuyNpcError::InsufficientGold);
        } else if self.inventory().total_of(item.currency()) < total_price {
            return Err(BuyNpcError::InsufficientQuantity);
        }
        if !self.current_map().content_code_is(item.npc_code()) {
            return Err(BuyNpcError::NpcNotFound);
        }
        Ok(())
    }

    pub fn npc_sell(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, SellNpcError> {
        self.can_npc_sell(item_code, quantity)?;
        Ok(self.handler().request_npc_sell(item_code, quantity)?)
    }

    fn can_npc_sell(&self, item_code: &str, quantity: u32) -> Result<(), SellNpcError> {
        if self.items.get(item_code).is_none() {
            return Err(SellNpcError::ItemNotFound);
        }
        let Some(item) = self.npcs.items().get(item_code) else {
            return Err(SellNpcError::ItemNotSalable);
        };
        if item.sell_price().is_none() {
            return Err(SellNpcError::ItemNotSalable);
        }
        if self
            .npcs
            .items()
            .get(item_code)
            .is_none_or(|item| !item.is_salable())
        {
            return Err(SellNpcError::ItemNotSalable);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(SellNpcError::InsufficientQuantity);
        }
        if !self.current_map().content_code_is(item.npc_code()) {
            return Err(SellNpcError::NpcNotFound);
        }
        Ok(())
    }

    pub fn give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &CharacterName,
    ) -> Result<(), GiveItemError> {
        self.can_give_item(items, character)?;
        Ok(self.handler().request_give_item(items, character)?)
    }

    pub fn can_give_item(
        &self,
        items: &[SimpleItemSchema],
        receiver: &CharacterName,
    ) -> Result<(), GiveItemError> {
        for item in items {
            if self.items.get(item.code()).is_none() {
                return Err(GiveItemError::ItemNotFound);
            }
            if self.inventory().total_of(item.code()) < item.quantity() {
                return Err(GiveItemError::InsufficientQuantity);
            }
        }
        let Some(receiver) = self.account().get_character(receiver) else {
            return Err(GiveItemError::CharacterNotFound);
        };
        if self.position() != receiver.position() {
            return Err(GiveItemError::CharacterNotFound);
        }
        if !receiver.inventory().has_room_for_all(items) {
            return Err(GiveItemError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn give_gold(&self, quantity: u32, character: &str) -> Result<(), GiveGoldError> {
        self.can_give_gold(quantity, character)?;
        Ok(self.handler().request_give_gold(quantity, character)?)
    }

    pub fn can_give_gold(&self, quantity: u32, character: &str) -> Result<(), GiveGoldError> {
        if self.gold() < quantity {
            return Err(GiveGoldError::InsufficientGold);
        }
        let Some(character) = self.account.get_character(character) else {
            return Err(GiveGoldError::CharacterNotFound);
        };
        if character.position() != self.position() {
            return Err(GiveGoldError::CharacterNotFound);
        }
        Ok(())
    }

    pub fn claim_pending_item(&self, id: &str) -> Result<(), ClaimPendingItemError> {
        self.can_claim_pending_item(id)?;
        Ok(self.handler().request_claim_pending_item(id)?)
    }

    fn can_claim_pending_item(&self, id: &str) -> Result<(), ClaimPendingItemError> {
        let Some(pending) = self
            .account()
            .pending_items()
            .into_iter()
            .find(|i| i.load().id() == id)
        else {
            return Err(ClaimPendingItemError::ItemNotFound);
        };
        if pending.load().is_claimed() {
            return Err(ClaimPendingItemError::AlreadyClaimed);
        }
        if !self.inventory().has_room_for_all(pending.load().items()) {
            return Err(ClaimPendingItemError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn ge_buy_order(
        &self,
        id: &str,
        quantity: u32,
    ) -> Result<GeTransactionSchema, GeBuyOrderError> {
        self.can_ge_buy_order(id, quantity)?;
        Ok(self.handler().request_ge_buy_order(id, quantity)?)
    }

    pub fn can_ge_buy_order(&self, id: &str, quantity: u32) -> Result<(), GeBuyOrderError> {
        let Some(order) = self.grand_exchange.get_order_by_id(id) else {
            return Err(GeBuyOrderError::OrderNotFound);
        };
        if order.r#type == GeOrderType::Buy
            && order.account.is_some_and(|a| a == self.account().name())
        {
            return Err(GeBuyOrderError::CannotTradeWithSelf);
        }
        if order.quantity < quantity {
            return Err(GeBuyOrderError::InsufficientQuantity);
        }
        if self.gold() < order.price * quantity {
            return Err(GeBuyOrderError::InsufficientGold);
        }
        if !self.inventory().has_room_for((&order.code, quantity)) {
            return Err(GeBuyOrderError::InsufficientInventorySpace);
        }
        if !self.current_map().is_grand_exchange() {
            return Err(GeBuyOrderError::NoGrandExchangeOnMap);
        }
        Ok(())
    }

    pub fn ge_create_order(
        &self,
        item_code: &str,
        quantity: u32,
        price: u32,
    ) -> Result<(), GeCreateOrderError> {
        self.can_ge_create_order(item_code, quantity, price)?;
        Ok(self
            .handler()
            .request_ge_create_order(item_code, quantity, price)?)
    }

    pub fn can_ge_create_order(
        &self,
        item_code: &str,
        quantity: u32,
        price: u32,
    ) -> Result<(), GeCreateOrderError> {
        let Some(item) = self.items.get(item_code) else {
            return Err(GeCreateOrderError::ItemNotFound);
        };
        if !item.is_tradeable() {
            return Err(GeCreateOrderError::ItemNotSalable);
        }
        if self.inventory().total_of(item.code()) < quantity {
            return Err(GeCreateOrderError::InsufficientQuantity);
        }
        if self.gold() < ((quantity * price) as f32 * 0.03).round() as u32 {
            return Err(GeCreateOrderError::InsufficientGold);
        }
        if !self.current_map().is_grand_exchange() {
            return Err(GeCreateOrderError::NoGrandExchangeOnMap);
        }
        Ok(())
    }

    pub fn ge_cancel_order(&self, id: &str) -> Result<GeTransactionSchema, GeCancelOrderError> {
        self.can_ge_cancel_order(id)?;
        Ok(self.handler().request_ge_cancel_order(id)?)
    }

    pub fn can_ge_cancel_order(&self, id: &str) -> Result<(), GeCancelOrderError> {
        let Some(order) = self.grand_exchange.get_order_by_id(id) else {
            return Err(GeCancelOrderError::OrderNotFound);
        };
        if order.account.is_some_and(|a| a != self.account().name()) {
            return Err(GeCancelOrderError::OrderNotOwned);
        }
        if !self.inventory().has_room_for((&order.code, order.quantity)) {
            return Err(GeCancelOrderError::InsufficientInventorySpace);
        }
        if !self.current_map().is_grand_exchange() {
            return Err(GeCancelOrderError::NoGrandExchangeOnMap);
        }
        Ok(())
    }

    fn invalidate_gear_cache(&self) {
        *self.gear_cache.lock().unwrap() = None;
    }

    #[must_use]
    pub fn gear(&self) -> Gear {
        if let Some(ref cached) = *self.gear_cache.lock().unwrap() {
            return cached.clone();
        }
        let gear = Gear {
            weapon: self.items.get(&self.equiped_in(Slot::Weapon)),
            shield: self.items.get(&self.equiped_in(Slot::Shield)),
            helmet: self.items.get(&self.equiped_in(Slot::Helmet)),
            body_armor: self.items.get(&self.equiped_in(Slot::BodyArmor)),
            leg_armor: self.items.get(&self.equiped_in(Slot::LegArmor)),
            boots: self.items.get(&self.equiped_in(Slot::Boots)),
            ring1: self.items.get(&self.equiped_in(Slot::Ring1)),
            ring2: self.items.get(&self.equiped_in(Slot::Ring2)),
            amulet: self.items.get(&self.equiped_in(Slot::Amulet)),
            artifact1: self.items.get(&self.equiped_in(Slot::Artifact1)),
            artifact2: self.items.get(&self.equiped_in(Slot::Artifact2)),
            artifact3: self.items.get(&self.equiped_in(Slot::Artifact3)),
            utility1: self.items.get(&self.equiped_in(Slot::Utility1)),
            utility2: self.items.get(&self.equiped_in(Slot::Utility2)),
            rune: self.items.get(&self.equiped_in(Slot::Rune)),
            bag: self.items.get(&self.equiped_in(Slot::Bag)),
            ..Default::default()
        };
        *self.gear_cache.lock().unwrap() = Some(gear.clone());
        gear
    }

    pub fn meets_conditions_for(&self, entity: &impl HasConditions) -> bool {
        entity.conditions().into_iter().flatten().all(|condition| {
            let value = condition.value as u32;
            // TODO: simplify this
            match condition.operator {
                ConditionOperator::Cost => {
                    if condition.code == GOLD {
                        self.gold() >= value
                    } else {
                        self.inventory().total_of(&condition.code) >= value
                    }
                }
                ConditionOperator::HasItem => self.has_equiped(&condition.code) >= value,
                ConditionOperator::AchievementUnlocked => self
                    .account
                    .get_achievement(&condition.code)
                    .is_some_and(AccountAchievement::is_completed),
                ConditionOperator::Eq => LevelConditionCode::from_str(&condition.code)
                    .is_ok_and(|code| self.skill_level(Skill::from(code)) == value),
                ConditionOperator::Ne => LevelConditionCode::from_str(&condition.code)
                    .is_ok_and(|code| self.skill_level(Skill::from(code)) != value),
                ConditionOperator::Gt => LevelConditionCode::from_str(&condition.code)
                    .is_ok_and(|code| self.skill_level(Skill::from(code)) > value),
                ConditionOperator::Lt => LevelConditionCode::from_str(&condition.code)
                    .is_ok_and(|code| self.skill_level(Skill::from(code)) < value),
            }
        })
    }

    #[must_use]
    pub fn account(&self) -> AccountClient {
        self.account.clone()
    }

    #[must_use]
    pub fn remaining_cooldown(&self) -> Duration {
        self.handler().remaining_cooldown()
    }

    #[must_use]
    pub fn current_map(&self) -> RawMap {
        self.maps
            .get_raw(&self.position())
            .expect("current position should always have a corresponding map")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CollectionClient,
        entities::{AccountAchievement, MapHandle, RawMap},
        test_utils::{ACCOUNT, MAPS, character},
    };
    use chrono::Utc;
    use itertools::Itertools;
    use openapi::models::{
        AccessSchema, AccountAchievementSchema, AchievementRewardsSchema, BankSchema,
        CharacterSchema, InteractionSchema, InventorySlotSchema, ItemSlot, MapAccessType,
        MapContentSchema, MapContentType, MapLayer, MapSchema,
    };
    use std::assert_matches;

    #[allow(clippy::unnecessary_wraps)]
    fn empty_inventory() -> Option<Vec<InventorySlotSchema>> {
        Some(
            (1..21)
                .map(|slot| InventorySlotSchema::new(slot, String::new(), 0))
                .collect(),
        )
    }

    fn default_schema() -> CharacterSchema {
        CharacterSchema {
            x: 0,
            y: 0,
            layer: MapLayer::Overworld,
            inventory_max_items: 100,
            inventory: empty_inventory(),
            ..Default::default()
        }
    }

    fn empty_bank_details() -> BankSchema {
        BankSchema {
            slots: 100,
            expansions: 0,
            next_expansion_cost: 100,
            gold: 0,
        }
    }

    #[test]
    fn can_move() {
        let char = character(default_schema());
        assert_matches!(char.can_move(0, 2), Ok(()));
        assert_matches!(char.can_move(0, 0), Err(MoveError::AlreadyOnMap));
        assert_matches!(char.can_move(1000, 0), Err(MoveError::MapNotFound));
        assert_matches!(char.can_move(3, 20), Err(MoveError::ConditionsNotMet));
    }

    #[test]
    fn can_move_achievement_unlocked() {
        let char = character(default_schema());
        assert_matches!(char.can_move(-2, 19), Err(MoveError::ConditionsNotMet));

        let achievement = AccountAchievement::new(AccountAchievementSchema {
            name: String::new(),
            code: "secure_the_island".into(),
            description: String::new(),
            points: 0,
            objectives: vec![],
            rewards: Box::new(AchievementRewardsSchema::new()),
            completed_at: Some(Utc::now().fixed_offset()),
        });
        ACCOUNT.add_achievement(achievement);

        assert_matches!(char.can_move(-2, 19), Ok(()));
    }

    #[test]
    fn can_fight() {
        let schema = CharacterSchema {
            y: 2,
            mining_level: 1,
            ..default_schema()
        };

        let char = character(schema.clone());
        assert_matches!(char.can_fight(None), Ok(()));

        let _partner = character(CharacterSchema {
            x: 3,
            y: -1,
            name: "partner".into(),
            ..default_schema()
        });
        let _partner2 = character(CharacterSchema {
            x: 3,
            y: -1,
            name: "partner2".into(),
            ..default_schema()
        });
        let char = character(CharacterSchema {
            x: 3,
            y: -1,
            ..default_schema()
        });
        assert_matches!(
            char.can_fight(Some(&["partner".into(), "partner2".into()])),
            Ok(())
        );

        let char = character(CharacterSchema {
            inventory_max_items: 1,
            ..schema.clone()
        });
        assert_matches!(
            char.can_fight(None),
            Err(FightError::InsufficientInventorySpace)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema.clone()
        });
        assert_matches!(char.can_fight(None), Err(FightError::NoMonsterOnMap));

        let char = character(schema.clone());
        assert_matches!(
            char.can_fight(Some(&["p".into(), "q".into()])),
            Err(FightError::MonsterIsNotABoss)
        );

        let char = character(CharacterSchema {
            x: 3,
            y: -1,
            ..schema
        });
        assert_matches!(
            char.can_fight(Some(&["nonexistent".into(), "also_missing".into()])),
            Err(FightError::CharacterNotFound)
        );
    }

    #[test]
    fn can_gather() {
        let schema = CharacterSchema {
            x: 2,
            mining_level: 1,
            ..default_schema()
        };

        let char = character(schema.clone());
        assert_matches!(char.can_gather(), Ok(()));

        let char = character(CharacterSchema {
            inventory_max_items: 1,
            ..schema.clone()
        });
        assert_matches!(
            char.can_gather(),
            Err(GatherError::InsufficientInventorySpace)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema.clone()
        });
        assert_matches!(char.can_gather(), Err(GatherError::NoResourceOnMap));

        let char = character(CharacterSchema {
            x: 1,
            y: 7,
            ..schema
        });
        assert_matches!(char.can_gather(), Err(GatherError::SkillLevelInsufficient));
    }

    #[test]
    fn can_use_item() {
        let char = character(CharacterSchema {
            level: 5,
            inventory: Some(vec![
                InventorySlotSchema::new(1, "cooked_chicken".into(), 1),
                InventorySlotSchema::new(2, "cooked_shrimp".into(), 1),
            ]),
            ..default_schema()
        });
        assert_matches!(char.can_use_item("cooked_chicken", 1), Ok(()));
        assert_matches!(char.can_use_item("unknown", 1), Err(UseError::ItemNotFound));
        assert_matches!(
            char.can_use_item("copper_ore", 1),
            Err(UseError::ItemNotConsumable)
        );
        assert_matches!(
            char.can_use_item("cooked_chicken", 5),
            Err(UseError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_use_item("cooked_shrimp", 1),
            Err(UseError::InsufficientCharacterLevel)
        );
    }

    #[test]
    fn can_craft() {
        let schema = CharacterSchema {
            x: 1,
            y: 1,
            cooking_level: 1,
            inventory: Some(vec![
                InventorySlotSchema::new(1, "gudgeon".into(), 1),
                InventorySlotSchema::new(2, "shrimp".into(), 1),
            ]),
            ..default_schema()
        };
        let char = character(schema.clone());
        assert_matches!(char.can_craft("cooked_gudgeon", 1), Ok(()));

        let char = character(schema.clone());
        assert_matches!(char.can_craft("unknown", 1), Err(CraftError::ItemNotFound));
        assert_matches!(
            char.can_craft("copper_ore", 1),
            Err(CraftError::ItemNotCraftable)
        );
        assert_matches!(
            char.can_craft("cooked_chicken", 1),
            Err(CraftError::InsufficientMaterials)
        );
        assert_matches!(
            char.can_craft("cooked_gudgeon", 5),
            Err(CraftError::InsufficientMaterials)
        );
        assert_matches!(
            char.can_craft("cooked_shrimp", 1),
            Err(CraftError::InsufficientSkillLevel)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema
        });
        assert_matches!(
            char.can_craft("cooked_gudgeon", 1),
            Err(CraftError::NoWorkshopOnMap)
        );
    }

    #[test]
    fn can_recycle() {
        let schema = CharacterSchema {
            x: 2,
            y: 1,
            weaponcrafting_level: 1,
            inventory: Some(vec![
                InventorySlotSchema::new(1, "copper_dagger".into(), 1),
                InventorySlotSchema::new(2, "iron_sword".into(), 1),
                InventorySlotSchema::new(3, "cooked_gudgeon".into(), 1),
            ]),
            ..default_schema()
        };
        let char = character(schema.clone());
        assert_matches!(char.can_recycle("copper_dagger", 1), Ok(()));

        let char = character(schema.clone());
        assert_matches!(
            char.can_recycle("random_item", 1),
            Err(RecycleError::ItemNotFound)
        );
        assert_matches!(
            char.can_recycle("cooked_gudgeon", 1),
            Err(RecycleError::ItemNotRecyclable)
        );
        assert_matches!(
            char.can_recycle("wooden_staff", 1),
            Err(RecycleError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_recycle("iron_sword", 1),
            Err(RecycleError::InsufficientSkillLevel)
        );

        let char = character(CharacterSchema {
            inventory_max_items: 3,
            ..schema.clone()
        });
        assert_matches!(
            char.can_recycle("copper_dagger", 1),
            Err(RecycleError::InsufficientInventorySpace)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema
        });
        assert_matches!(
            char.can_recycle("copper_dagger", 1),
            Err(RecycleError::NoWorkshopOnMap)
        );
    }

    #[test]
    fn can_delete() {
        let char = character(CharacterSchema {
            inventory: Some(vec![InventorySlotSchema {
                slot: 1,
                code: "copper_dagger".into(),
                quantity: 1,
            }]),
            ..default_schema()
        });
        assert_matches!(char.can_delete("copper_dagger", 1), Ok(()));
        assert_matches!(
            char.can_delete("random_item", 1),
            Err(DeleteError::ItemNotFound)
        );
        assert_matches!(
            char.can_delete("copper_dagger", 2),
            Err(DeleteError::InsufficientQuantity)
        );
    }

    #[test]
    fn can_withdraw() {
        let char = character(CharacterSchema {
            x: 4,
            y: 1,
            ..default_schema()
        });
        char.bank.set_content(vec![
            SimpleItemSchema::new("copper_dagger".into(), 1),
            SimpleItemSchema::new("iron_sword".into(), 101),
        ]);

        assert_matches!(
            char.can_withdraw_items(&[SimpleItemSchema::new("iron_sword".into(), 10)]),
            Ok(())
        );
        assert_matches!(
            char.can_withdraw_items(&[SimpleItemSchema::new("random_item".into(), 1)]),
            // TODO: add ItemNotFound variant
            Err(WithdrawError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_withdraw_items(&[SimpleItemSchema::new("copper_dagger".into(), 2)]),
            Err(WithdrawError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_withdraw_items(&[SimpleItemSchema::new("iron_sword".into(), 101)]),
            Err(WithdrawError::InsufficientInventorySpace)
        );

        let char = character(default_schema());
        char.bank
            .set_content(vec![SimpleItemSchema::new("copper_dagger".into(), 1)]);
        assert_matches!(
            char.can_withdraw_items(&[SimpleItemSchema::new("copper_dagger".into(), 1)]),
            Err(WithdrawError::NoBankOnMap)
        );
    }

    #[test]
    fn can_transition() {
        let schema = CharacterSchema {
            x: -4,
            y: 9,
            ..default_schema()
        };

        let char = character(CharacterSchema {
            gold: 5000,
            ..schema.clone()
        });
        assert_matches!(char.can_transition(), Ok(()));

        let char = character(schema.clone());
        assert_matches!(
            char.can_transition(),
            Err(TransitionError::ConditionsNotMet)
        );
        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema
        });
        assert_matches!(
            char.can_transition(),
            Err(TransitionError::TransitionNotFound)
        );
    }

    #[test]
    fn can_transition_cost_non_gold() {
        let char = character(CharacterSchema {
            x: 7,
            y: 3,
            layer: MapLayer::Underground,
            inventory: Some(vec![InventorySlotSchema::new(
                1,
                "sonnengott_key".into(),
                1,
            )]),
            ..default_schema()
        });
        assert_matches!(char.can_transition(), Ok(()));

        let char = character(CharacterSchema {
            x: 7,
            y: 3,
            layer: MapLayer::Underground,
            ..default_schema()
        });
        assert_matches!(
            char.can_transition(),
            Err(TransitionError::ConditionsNotMet)
        );
    }

    #[test]
    fn can_transition_has_item() {
        let char = character(CharacterSchema {
            x: 0,
            y: 13,
            weapon_slot: "cultist_cloak".into(),
            ..default_schema()
        });
        assert_matches!(char.can_transition(), Ok(()));

        let char = character(CharacterSchema {
            x: 0,
            y: 13,
            ..default_schema()
        });
        assert_matches!(
            char.can_transition(),
            Err(TransitionError::ConditionsNotMet)
        );
    }

    #[test]
    fn can_deposit_items() {
        let char = character(CharacterSchema {
            x: 4,
            y: 1,
            inventory: Some(vec![InventorySlotSchema::new(0, "copper_ore".into(), 5)]),
            ..default_schema()
        });
        char.bank.set_details(empty_bank_details());
        char.bank.set_content(vec![]);

        assert_matches!(
            char.can_deposit_items(&[SimpleItemSchema::new("copper_ore".into(), 1)]),
            Ok(())
        );
        assert_matches!(
            char.can_deposit_items(&[SimpleItemSchema::new("none".into(), 1)]),
            Err(DepositError::ItemNotFound)
        );
        assert_matches!(
            char.can_deposit_items(&[SimpleItemSchema {
                code: "copper_ore".into(),
                quantity: 10,
            }]),
            Err(DepositError::InsufficientQuantity)
        );

        char.bank.set_details(BankSchema {
            slots: 0,
            ..empty_bank_details()
        });
        assert_matches!(
            char.can_deposit_items(&[SimpleItemSchema {
                code: "copper_ore".into(),
                quantity: 1,
            }]),
            Err(DepositError::InsufficientBankSpace)
        );

        let char_no_bank = character(CharacterSchema {
            inventory: Some(vec![InventorySlotSchema::new(0, "copper_ore".into(), 5)]),
            ..default_schema()
        });
        char_no_bank.bank.set_details(empty_bank_details());
        char_no_bank.bank.set_content(vec![]);
        assert_matches!(
            char_no_bank.can_deposit_items(&[SimpleItemSchema::new("copper_ore".into(), 1)]),
            Err(DepositError::NoBankOnMap)
        );
    }

    #[test]
    fn can_deposit_gold() {
        let schema = CharacterSchema {
            x: 4,
            y: 1,
            gold: 0,
            ..default_schema()
        };

        let char = character(CharacterSchema {
            gold: 100,
            ..schema.clone()
        });
        assert_matches!(char.can_deposit_gold(100), Ok(()));

        let char = character(schema.clone());
        assert_matches!(
            char.can_deposit_gold(100),
            Err(GoldDepositError::InsufficientGold)
        );

        let char_no_bank = character(CharacterSchema {
            x: 0,
            y: 0,
            gold: 100,
            ..schema
        });
        assert_matches!(
            char_no_bank.can_deposit_gold(100),
            Err(GoldDepositError::NoBankOnMap)
        );
    }

    #[test]
    fn can_withdraw_gold() {
        let schema = CharacterSchema {
            x: 4,
            y: 1,
            ..default_schema()
        };

        let char = character(schema.clone());
        char.bank.set_details(BankSchema {
            gold: 100,
            ..empty_bank_details()
        });
        assert_matches!(char.can_withdraw_gold(100), Ok(()));

        let char = character(schema.clone());
        char.bank.set_details(empty_bank_details());
        assert_matches!(
            char.can_withdraw_gold(100),
            Err(GoldWithdrawError::InsufficientGold)
        );

        let char_no_bank = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema
        });
        char_no_bank.bank.set_details(BankSchema {
            gold: 100,
            ..empty_bank_details()
        });
        assert_matches!(
            char_no_bank.can_withdraw_gold(100),
            Err(GoldWithdrawError::NoBankOnMap)
        );
    }

    #[test]
    fn can_expand_bank() {
        let schema = CharacterSchema {
            x: 4,
            y: 1,
            gold: 0,
            ..default_schema()
        };
        let bank = BankSchema {
            next_expansion_cost: 5000,
            ..empty_bank_details()
        };

        let char = character(CharacterSchema {
            gold: 5000,
            ..schema.clone()
        });
        char.bank.set_details(bank.clone());
        assert_matches!(char.can_expand_bank(), Ok(()));

        let char = character(schema.clone());
        char.bank.set_details(bank.clone());
        assert_matches!(
            char.can_expand_bank(),
            Err(BankExpansionError::InsufficientGold)
        );

        let char_no_bank = character(CharacterSchema {
            x: 0,
            y: 0,
            gold: 5000,
            ..schema
        });
        char_no_bank.bank.set_details(bank);
        assert_matches!(
            char_no_bank.can_expand_bank(),
            Err(BankExpansionError::NoBankOnMap)
        );
    }

    #[test]
    fn can_accept_task() {
        let schema = CharacterSchema {
            x: 1,
            y: 2,
            ..default_schema()
        };

        let char = character(schema.clone());
        assert_matches!(char.can_accept_task(), Ok(()));

        let char = character(CharacterSchema {
            task: "chicken".into(),
            task_type: "monsters".into(),
            ..schema.clone()
        });
        assert_matches!(
            char.can_accept_task(),
            Err(TaskAcceptationError::TaskAlreadyInProgress)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema
        });
        assert_matches!(
            char.can_accept_task(),
            Err(TaskAcceptationError::NoTasksMasterOnMap)
        );
    }

    #[test]
    fn can_cancel_task() {
        let schema = CharacterSchema {
            x: 1,
            y: 2,
            ..default_schema()
        };

        let char = character(CharacterSchema {
            task: "chicken".into(),
            task_type: "monsters".into(),
            inventory: Some(vec![InventorySlotSchema {
                slot: 0,
                code: TASKS_COIN.into(),
                quantity: 1,
            }]),
            ..schema.clone()
        });
        assert_matches!(char.can_cancel_task(), Ok(()));

        let char = character(schema.clone());
        assert_matches!(
            char.can_cancel_task(),
            Err(TaskCancellationError::NoCurrentTask)
        );

        let char = character(CharacterSchema {
            task: "chicken".into(),
            task_type: "monsters".into(),
            ..schema.clone()
        });
        assert_matches!(
            char.can_cancel_task(),
            Err(TaskCancellationError::InsufficientTasksCoinQuantity)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            task: "chicken".into(),
            task_type: "monsters".into(),
            inventory: Some(vec![InventorySlotSchema {
                slot: 0,
                code: TASKS_COIN.into(),
                quantity: 1,
            }]),
            ..schema
        });
        assert_matches!(
            char.can_cancel_task(),
            Err(TaskCancellationError::WrongOrNoTasksMasterOnMap)
        );
    }

    #[test]
    fn can_trade_task_item() {
        let schema = CharacterSchema {
            x: 4,
            y: 13,
            task: "copper_ore".into(),
            task_type: "items".into(),
            task_progress: 0,
            task_total: 100,
            inventory: Some(vec![InventorySlotSchema::new(0, "copper_ore".into(), 50)]),
            ..default_schema()
        };

        let char = character(schema.clone());
        assert_matches!(char.can_trade_task_item("copper_ore", 10), Ok(()));

        let char = character(schema.clone());
        assert_matches!(
            char.can_trade_task_item("none", 1),
            Err(TaskTradeError::ItemNotFound)
        );
        assert_matches!(
            char.can_trade_task_item("iron_ore", 1),
            Err(TaskTradeError::WrongTask)
        );
        assert_matches!(
            char.can_trade_task_item("copper_ore", 100),
            Err(TaskTradeError::InsufficientQuantity)
        );

        let char = character(CharacterSchema {
            task_progress: 95,
            inventory: Some(vec![InventorySlotSchema::new(0, "copper_ore".into(), 10)]),
            ..schema.clone()
        });
        assert_matches!(
            char.can_trade_task_item("copper_ore", 10),
            Err(TaskTradeError::SuperfluousQuantity)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            ..schema
        });
        assert_matches!(
            char.can_trade_task_item("copper_ore", 10),
            Err(TaskTradeError::WrongOrNoTasksMasterOnMap)
        );
    }

    #[test]
    fn can_complete_task() {
        let schema = CharacterSchema {
            x: 1,
            y: 2,
            ..default_schema()
        };

        let char = character(CharacterSchema {
            task: "chicken".into(),
            task_type: "monsters".into(),
            task_progress: 100,
            task_total: 100,
            ..schema.clone()
        });
        assert_matches!(char.can_complete_task(), Ok(()));

        let char = character(schema.clone());
        assert_matches!(
            char.can_complete_task(),
            Err(TaskCompletionError::NoCurrentTask)
        );

        let char = character(CharacterSchema {
            task: "chicken".into(),
            task_type: "monsters".into(),
            task_progress: 0,
            task_total: 100,
            ..schema.clone()
        });
        assert_matches!(
            char.can_complete_task(),
            Err(TaskCompletionError::TaskNotFullfilled)
        );

        let char = character(CharacterSchema {
            x: 4,
            y: 13,
            task: "copper_ore".into(),
            task_type: "items".into(),
            task_progress: 100,
            task_total: 100,
            inventory_max_items: 1,
            ..schema.clone()
        });
        assert_matches!(
            char.can_complete_task(),
            Err(TaskCompletionError::InsufficientInventorySpace)
        );

        let char = character(CharacterSchema {
            x: 0,
            y: 0,
            task: "chicken".into(),
            task_type: "monsters".into(),
            task_progress: 100,
            task_total: 100,
            ..schema
        });
        assert_matches!(
            char.can_complete_task(),
            Err(TaskCompletionError::WrongOrNoTasksMasterOnMap)
        );
    }

    #[test]
    fn can_exchange_tasks_coins() {
        let schema = default_schema();

        let mut modified_schema = schema.clone();
        modified_schema
            .inventory
            .as_mut()
            .unwrap()
            .push(InventorySlotSchema::new(1, TASKS_COIN.into(), 6));
        let char = character(CharacterSchema {
            x: 1,
            y: 2,
            ..modified_schema.clone()
        });
        assert_matches!(char.can_exchange_tasks_coins(), Ok(()));

        let char = character(CharacterSchema {
            inventory: Some(vec![InventorySlotSchema::new(1, TASKS_COIN.into(), 3)]),
            ..schema.clone()
        });
        assert_matches!(
            char.can_exchange_tasks_coins(),
            Err(TasksCoinExchangeError::InsufficientTasksCoinQuantity)
        );

        let char = character(CharacterSchema {
            x: 1,
            y: 2,
            inventory: Some({
                let mut slots = (1..21)
                    .map(|slot| InventorySlotSchema::new(slot, "copper_ore".into(), 1))
                    .collect_vec();
                slots.push(InventorySlotSchema::new(1, TASKS_COIN.into(), 10));
                slots
            }),
            inventory_max_items: 20,
            ..schema
        });
        assert_matches!(
            char.can_exchange_tasks_coins(),
            Err(TasksCoinExchangeError::InsufficientInventorySpace)
        );

        let char = character(modified_schema);
        assert_matches!(
            char.can_exchange_tasks_coins(),
            Err(TasksCoinExchangeError::NoTasksMasterOnMap)
        );
    }

    #[test]
    fn can_equip() {
        let char = character(CharacterSchema {
            level: 5,
            inventory: Some(vec![
                InventorySlotSchema::new(1, "small_health_potion".into(), 1),
                InventorySlotSchema::new(2, "iron_sword".into(), 1),
                InventorySlotSchema::new(3, "wooden_stick".into(), 1),
                InventorySlotSchema::new(4, "copper_boots".into(), 1),
                InventorySlotSchema::new(4, "copper_dagger".into(), 1),
            ]),
            weapon_slot: "copper_dagger".into(),
            ..default_schema()
        });

        assert_matches!(
            char.can_equip(&[EquipSchema::new("copper_boots".into(), ItemSlot::Boots)]),
            Ok(())
        );
        assert_matches!(
            char.can_equip(&[EquipSchema::new("none".into(), ItemSlot::Weapon)]),
            Err(EquipError::ItemNotFound)
        );
        assert_matches!(
            char.can_equip(&[EquipSchema {
                code: "small_health_potion".into(),
                slot: ItemSlot::Weapon,
                quantity: Some(5),
            }]),
            Err(EquipError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_equip(&[EquipSchema::new("iron_sword".into(), ItemSlot::Weapon)]),
            Err(EquipError::ConditionsNotMet)
        );
        assert_matches!(
            char.can_equip(&[EquipSchema::new("copper_dagger".into(), ItemSlot::Weapon)]),
            Err(EquipError::ItemAlreadyEquiped)
        );
        assert_matches!(
            char.can_equip(&[EquipSchema::new("wooden_stick".into(), ItemSlot::Weapon,)]),
            Err(EquipError::SlotNotEmpty)
        );
    }

    #[test]
    fn can_unequip() {
        let char = character(CharacterSchema {
            inventory_max_items: 50,
            level: 10,
            hp: 5,
            max_hp: 100,
            utility1_slot: "small_health_potion".into(),
            utility1_slot_quantity: 5,
            helmet_slot: "copper_helmet".into(),
            weapon_slot: "copper_dagger".into(),
            utility2_slot: "water_boost_potion".into(),
            utility2_slot_quantity: 100,
            ..default_schema()
        });
        assert_matches!(
            char.can_unequip(&[UnequipSchema::new(ItemSlot::Weapon)]),
            Ok(())
        );
        assert_matches!(
            char.can_unequip(&[UnequipSchema::new(ItemSlot::Boots)]),
            Err(UnequipError::SlotEmpty)
        );
        assert_matches!(
            char.can_unequip(&[UnequipSchema {
                slot: ItemSlot::Utility1,
                quantity: Some(10),
            }]),
            Err(UnequipError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_unequip(&[UnequipSchema::new(ItemSlot::Helmet)]),
            Err(UnequipError::InsufficientHealth)
        );
        assert_matches!(
            char.can_unequip(&[UnequipSchema {
                slot: ItemSlot::Utility2,
                quantity: Some(100)
            }]),
            Err(UnequipError::InsufficientInventorySpace)
        );
    }

    #[test]
    fn can_npc_buy() {
        let schema = CharacterSchema {
            gold: 100,
            inventory: Some(vec![InventorySlotSchema::new(1, "wool".into(), 3)]),
            ..default_schema()
        };

        let char = character(CharacterSchema {
            x: 3,
            y: 3,
            gold: 1000,
            ..schema.clone()
        });
        assert_matches!(char.can_npc_buy("cloth", 1), Ok(()));

        let char = character(schema);
        assert_matches!(char.can_npc_buy("none", 1), Err(BuyNpcError::ItemNotFound));
        assert_matches!(
            char.can_npc_buy("holey_boot", 1),
            Err(BuyNpcError::ItemNotBuyable)
        );
        assert_matches!(
            char.can_npc_buy("gold_ore", 200),
            Err(BuyNpcError::InsufficientGold)
        );
        assert_matches!(
            char.can_npc_buy("cloth", 2),
            Err(BuyNpcError::InsufficientQuantity)
        );
        assert_matches!(char.can_npc_buy("cloth", 1), Err(BuyNpcError::NpcNotFound));
    }

    #[test]
    fn can_npc_sell() {
        let handle: MapHandle =
            CollectionClient::get(&*MAPS, &(MapLayer::Overworld, 2, 8)).unwrap();
        handle.store(RawMap::new(MapSchema::new(
            0,
            "test_npc_map".into(),
            "default".into(),
            2,
            8,
            MapLayer::Overworld,
            AccessSchema::new(MapAccessType::Standard),
            InteractionSchema {
                content: Some(Box::new(MapContentSchema::new(
                    MapContentType::Npc,
                    "gemstone_merchant".into(),
                ))),
                transition: None,
            },
        )));
        let char = character(CharacterSchema {
            x: 2,
            y: 8,
            inventory: Some(vec![InventorySlotSchema::new(1, "copper_ore".into(), 5)]),
            ..default_schema()
        });
        assert_matches!(char.can_npc_sell("copper_ore", 1), Ok(()));

        let char = character(CharacterSchema {
            inventory: Some(vec![InventorySlotSchema::new(1, "gold_ore".into(), 5)]),
            ..default_schema()
        });
        assert_matches!(
            char.can_npc_sell("nonexistent_item", 1),
            Err(SellNpcError::ItemNotFound)
        );
        assert_matches!(
            char.can_npc_sell("burn_rune", 1),
            Err(SellNpcError::ItemNotSalable)
        );
        assert_matches!(
            char.can_npc_sell("gold_ore", 10),
            Err(SellNpcError::InsufficientQuantity)
        );
    }

    #[test]
    fn can_ge_create_order() {
        let schema = CharacterSchema {
            gold: 50,
            inventory: Some(vec![
                InventorySlotSchema::new(1, "gold_ore".into(), 5),
                InventorySlotSchema::new(2, "tasks_coin".into(), 1),
            ]),
            ..default_schema()
        };

        let char = character(CharacterSchema {
            x: 5,
            y: 1,
            ..schema.clone()
        });
        assert_matches!(char.can_ge_create_order("gold_ore", 1, 1), Ok(()));

        let char = character(schema);
        assert_matches!(
            char.can_ge_create_order("none", 1, 1),
            Err(GeCreateOrderError::ItemNotFound)
        );
        assert_matches!(
            char.can_ge_create_order("tasks_coin", 1, 1),
            Err(GeCreateOrderError::ItemNotSalable)
        );
        assert_matches!(
            char.can_ge_create_order("gold_ore", 10, 1),
            Err(GeCreateOrderError::InsufficientQuantity)
        );
        assert_matches!(
            char.can_ge_create_order("gold_ore", 1, 1_000_000_000),
            Err(GeCreateOrderError::InsufficientGold)
        );
        assert_matches!(
            char.can_ge_create_order("gold_ore", 1, 1),
            Err(GeCreateOrderError::NoGrandExchangeOnMap)
        );
    }

    #[test]
    fn can_give_gold() {
        let char = character(CharacterSchema {
            gold: 100,
            ..default_schema()
        });
        let _receiver = character(CharacterSchema {
            name: "receiver".into(),
            ..default_schema()
        });
        assert_matches!(char.can_give_gold(50, "receiver"), Ok(()));
        assert_matches!(
            char.can_give_gold(50, "nonexistent"),
            Err(GiveGoldError::CharacterNotFound)
        );

        let no_gold = character(CharacterSchema {
            gold: 0,
            ..default_schema()
        });
        assert_matches!(
            no_gold.can_give_gold(50, "receiver"),
            Err(GiveGoldError::InsufficientGold)
        );

        let far_away = character(CharacterSchema {
            gold: 100,
            x: 1,
            y: 2,
            ..default_schema()
        });
        assert_matches!(
            far_away.can_give_gold(50, "receiver"),
            Err(GiveGoldError::CharacterNotFound)
        );
    }

    #[test]
    fn can_give_item() {
        let char = character(CharacterSchema {
            inventory: Some(vec![InventorySlotSchema::new(1, "copper_ore".into(), 5)]),
            ..default_schema()
        });
        let _receiver = character(CharacterSchema {
            name: "receiver".into(),
            ..default_schema()
        });
        let _full_receiver = character(CharacterSchema {
            name: "full_receiver".into(),
            inventory_max_items: 1,
            inventory: Some(vec![InventorySlotSchema::new(1, "iron_sword".into(), 1)]),
            ..default_schema()
        });

        assert_matches!(
            char.can_give_item(
                &[SimpleItemSchema::new("copper_ore".into(), 3)],
                &"receiver".into()
            ),
            Ok(())
        );
        assert_matches!(
            char.can_give_item(
                &[SimpleItemSchema::new("copper_ore".into(), 1)],
                &"nonexistent".into()
            ),
            Err(GiveItemError::CharacterNotFound)
        );
        assert_matches!(
            char.can_give_item(
                &[SimpleItemSchema::new("copper_ore".into(), 1)],
                &"full_receiver".into()
            ),
            Err(GiveItemError::InsufficientInventorySpace)
        );

        let empty = character(default_schema());
        assert_matches!(
            empty.can_give_item(
                &[SimpleItemSchema::new("unknown".into(), 1)],
                &"receiver".into()
            ),
            Err(GiveItemError::ItemNotFound)
        );

        let small_inv = character(CharacterSchema {
            inventory: Some(vec![InventorySlotSchema::new(1, "copper_ore".into(), 2)]),
            ..default_schema()
        });
        assert_matches!(
            small_inv.can_give_item(
                &[SimpleItemSchema::new("copper_ore".into(), 5)],
                &"receiver".into()
            ),
            Err(GiveItemError::InsufficientQuantity)
        );

        let far_away = character(CharacterSchema {
            x: 1,
            y: 2,
            inventory: Some(vec![InventorySlotSchema::new(1, "copper_ore".into(), 5)]),
            ..default_schema()
        });
        assert_matches!(
            far_away.can_give_item(
                &[SimpleItemSchema::new("copper_ore".into(), 1)],
                &"receiver".into()
            ),
            Err(GiveItemError::CharacterNotFound)
        );
    }
}
