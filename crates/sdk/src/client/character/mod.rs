use crate::{
    AccountClient, Code, CollectionClient, GOLD, Gear, HasConditions, ItemContainer, Level,
    LimitedContainer, SlotLimited, SpaceLimited, TASK_EXCHANGE_PRICE, TASKS_COIN, TasksClient,
    character::{
        error::{
            GeBuyOrderError, GeCancelOrderError, GeCreateOrderError, GiveGoldError, GiveItemError,
            TransitionError,
        },
        inventory::Inventory,
    },
    client::{
        bank::{Bank, BankClient},
        character::{
            error::{
                BankExpansionError, BuyNpcError, CraftError, DeleteError, DepositError, EquipError,
                FightError, GatherError, GoldDepositError, GoldWithdrawError, MoveError,
                RecycleError, RestError, SellNpcError, TaskAcceptationError, TaskCancellationError,
                TaskCompletionError, TaskTradeError, TasksCoinExchangeError, UnequipError,
                UseError, WithdrawError,
            },
            request_handler::CharacterRequestHandler,
        },
        items::{ItemsClient, LevelConditionCode},
        maps::MapsClient,
        monsters::MonstersClient,
        npcs::NpcsClient,
        resources::ResourcesClient,
        server::ServerClient,
    },
    entities::Map,
    gear::Slot,
    grand_exchange::GrandExchangeClient,
    simulator::HasEffects,
    skill::Skill,
};
use api::ArtifactApi;
use chrono::{DateTime, Utc};
use openapi::models::{
    CharacterFightSchema, CharacterSchema, ConditionOperator, GeTransactionSchema, MapContentType,
    MapLayer, NpcItemTransactionSchema, RecyclingItemsSchema, RewardsSchema, SimpleItemSchema,
    SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, TaskType,
};
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};
use strum::IntoEnumIterator;

pub use inventory::InventoryClient;

mod request_handler;

pub mod action;
pub mod error;
pub mod inventory;

pub type CharacterData = Arc<RwLock<Arc<CharacterSchema>>>;

#[derive(Default, Debug)]
pub struct CharacterClient {
    pub id: usize,
    inner: CharacterRequestHandler,
    account: Arc<AccountClient>,
    bank: Arc<BankClient>,
    items: Arc<ItemsClient>,
    resources: Arc<ResourcesClient>,
    monsters: Arc<MonstersClient>,
    maps: Arc<MapsClient>,
    npcs: Arc<NpcsClient>,
    tasks: Arc<TasksClient>,
    grand_exchange: Arc<GrandExchangeClient>,
}

impl CharacterClient {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: usize,
        data: CharacterData,
        account: Arc<AccountClient>,
        items: Arc<ItemsClient>,
        resources: Arc<ResourcesClient>,
        monsters: Arc<MonstersClient>,
        maps: Arc<MapsClient>,
        npcs: Arc<NpcsClient>,
        tasks: Arc<TasksClient>,
        grand_exchange: Arc<GrandExchangeClient>,
        server: Arc<ServerClient>,
        api: Arc<ArtifactApi>,
    ) -> Self {
        Self {
            id,
            inner: CharacterRequestHandler::new(api, data.clone(), account.clone(), server.clone()),
            bank: account.bank.clone(),
            account,
            items,
            resources,
            monsters,
            maps,
            npcs,
            tasks,
            grand_exchange,
        }
    }

    pub fn r#move(&self, x: i32, y: i32) -> Result<Map, MoveError> {
        self.can_move(x, y)?;
        Ok(self.inner.request_move(x, y)?)
    }

    pub fn can_move(&self, x: i32, y: i32) -> Result<(), MoveError> {
        if self.position() == (self.position().0, x, y) {
            return Err(MoveError::AlreadyOnMap);
        }
        let Some(map) = self.maps.get(self.position().0, x, y) else {
            return Err(MoveError::MapNotFound);
        };
        if map.is_blocked() || !self.meets_conditions_for(map.access()) {
            return Err(MoveError::ConditionsNotMet);
        }
        Ok(())
    }

    pub fn transition(&self) -> Result<Map, TransitionError> {
        self.can_transition()?;
        Ok(self.inner.request_transition()?)
    }

    pub fn can_transition(&self) -> Result<(), TransitionError> {
        let map = self.current_map();
        let Some(ref transition) = map.interactions().transition else {
            return Err(TransitionError::TransitionNotFound);
        };
        if !self.meets_conditions_for(transition.as_ref()) {
            return Err(TransitionError::ConditionsNotMet);
        }
        Ok(())
    }

    pub fn fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, FightError> {
        self.can_fight(participants)?;
        Ok(self.inner.request_fight(participants)?)
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
        if !participants.is_empty() && monster.is_boss() {
            return Err(FightError::MonsterIsNotABoss);
        }
        for name in participants.iter() {
            let Some(char) = self.account.get_character_by_name(name) else {
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

    pub fn gather(&self) -> Result<SkillDataSchema, GatherError> {
        self.can_gather()?;
        Ok(self.inner.request_gather()?)
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
        if self.health() < self.max_health() {
            return Ok(self.inner.request_rest()?);
        }
        Ok(0)
    }

    pub fn craft(&self, item_code: &str, quantity: u32) -> Result<SkillInfoSchema, CraftError> {
        self.can_craft(item_code, quantity)?;
        Ok(self.inner.request_craft(item_code, quantity)?)
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
        if !self.inventory().contains_multiple(&item.mats_for(quantity)) {
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
        Ok(self.inner.request_recycle(item_code, quantity)?)
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
        Ok(self.inner.request_delete(item_code, quantity)?)
    }

    pub fn can_delete(&self, item_code: &str, quantity: u32) -> Result<(), DeleteError> {
        if self.items.get(item_code).is_none() {
            return Err(DeleteError::ItemNotFound);
        };
        if self.inventory().total_of(item_code) < quantity {
            return Err(DeleteError::InsufficientQuantity);
        }
        Ok(())
    }

    pub fn deposit_item(&self, items: &[SimpleItemSchema]) -> Result<(), DepositError> {
        self.can_deposit_items(items)?;
        Ok(self.inner.request_deposit_item(items)?)
    }

    pub fn can_deposit_items(&self, items: &[SimpleItemSchema]) -> Result<(), DepositError> {
        for item in items.iter() {
            if self.items.get(&item.code).is_none() {
                return Err(DepositError::ItemNotFound);
            };
            if self.inventory().total_of(&item.code) < item.quantity {
                return Err(DepositError::InsufficientQuantity);
            }
        }
        if !self.bank.has_room_for_multiple(items) {
            return Err(DepositError::InsufficientBankSpace);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(DepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), WithdrawError> {
        self.can_withdraw_items(items)?;
        Ok(self.inner.request_withdraw_item(items)?)
    }

    pub fn can_withdraw_items(&self, items: &[SimpleItemSchema]) -> Result<(), WithdrawError> {
        if items
            .iter()
            .any(|i| self.bank.total_of(&i.code) < i.quantity)
        {
            return Err(WithdrawError::InsufficientQuantity);
        };
        if !self.inventory().has_room_for_multiple(items) {
            return Err(WithdrawError::InsufficientInventorySpace);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(WithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn deposit_gold(&self, quantity: u32) -> Result<u32, GoldDepositError> {
        self.can_deposit_gold(quantity)?;
        Ok(self.inner.request_deposit_gold(quantity)?)
    }

    pub fn can_deposit_gold(&self, quantity: u32) -> Result<(), GoldDepositError> {
        if self.gold() < quantity {
            return Err(GoldDepositError::InsufficientGold);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(GoldDepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn withdraw_gold(&self, quantity: u32) -> Result<u32, GoldWithdrawError> {
        self.can_withdraw_gold(quantity)?;
        Ok(self.inner.request_withdraw_gold(quantity)?)
    }

    pub fn can_withdraw_gold(&self, quantity: u32) -> Result<(), GoldWithdrawError> {
        if self.bank.gold() < quantity {
            return Err(GoldWithdrawError::InsufficientGold);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(GoldWithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn expand_bank(&self) -> Result<u32, BankExpansionError> {
        self.can_expand_bank()?;
        Ok(self.inner.request_expand_bank()?)
    }

    pub fn can_expand_bank(&self) -> Result<(), BankExpansionError> {
        if self.gold() < self.bank.next_expansion_cost() {
            return Err(BankExpansionError::InsufficientGold);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(BankExpansionError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn equip(&self, item_code: &str, slot: Slot, quantity: u32) -> Result<(), EquipError> {
        self.can_equip(item_code, slot, quantity)?;
        Ok(self.inner.request_equip(item_code, slot, quantity)?)
    }

    pub fn can_equip(&self, item_code: &str, slot: Slot, quantity: u32) -> Result<(), EquipError> {
        let Some(item) = self.items.get(item_code) else {
            return Err(EquipError::ItemNotFound);
        };
        if self.inventory().total_of(item_code) < quantity {
            return Err(EquipError::InsufficientQuantity);
        }
        if let Some(equiped) = self.items.get(&self.equiped_in(slot)) {
            if equiped.code() != item_code {
                return Err(EquipError::SlotNotEmpty);
            };
            if slot.max_quantity() <= 1 {
                return Err(EquipError::ItemAlreadyEquiped);
            } else if self.quantity_in_slot(slot) + quantity > slot.max_quantity() {
                return Err(EquipError::QuantityGreaterThanSlotMaxixum);
            }
        }
        if !self.meets_conditions_for(&item) {
            return Err(EquipError::ConditionsNotMet);
        }
        if self.inventory().free_space() as i32 + item.inventory_space() <= 0 {
            return Err(EquipError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn unequip(&self, slot: Slot, quantity: u32) -> Result<(), UnequipError> {
        self.can_unequip(slot, quantity)?;
        Ok(self.inner.request_unequip(slot, quantity)?)
    }

    pub fn can_unequip(&self, slot: Slot, quantity: u32) -> Result<(), UnequipError> {
        let Some(equiped) = self.items.get(&self.equiped_in(slot)) else {
            return Err(UnequipError::SlotEmpty);
        };
        if self.health() <= equiped.health() {
            return Err(UnequipError::InsufficientHealth);
        }
        if self.quantity_in_slot(slot) < quantity {
            return Err(UnequipError::InsufficientQuantity);
        }
        if !self.inventory().has_room_for(equiped.code(), quantity) {
            return Err(UnequipError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn use_item(&self, item_code: &str, quantity: u32) -> Result<(), UseError> {
        self.can_use_item(item_code, quantity)?;
        Ok(self.inner.request_use_item(item_code, quantity)?)
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
        Ok(self.inner.request_accept_task()?)
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
        Ok(self.inner.request_cancel_task()?)
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
        Ok(self.inner.request_trade_task_item(item_code, quantity)?)
    }

    pub fn can_trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<(), TaskTradeError> {
        if self.items.get(item_code).is_none() {
            return Err(TaskTradeError::ItemNotFound);
        };
        if item_code != self.task() {
            return Err(TaskTradeError::WrongTask);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(TaskTradeError::InsufficientQuantity);
        }
        if self.task_missing() < quantity {
            return Err(TaskTradeError::SuperfluousQuantity);
        }
        if !self
            .current_map()
            .content_type_is(MapContentType::TasksMaster)
            || !self.current_map().content_code_is("items")
        {
            return Err(TaskTradeError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn complete_task(&self) -> Result<RewardsSchema, TaskCompletionError> {
        self.can_complete_task()?;
        Ok(self.inner.request_complete_task()?)
    }

    pub fn can_complete_task(&self) -> Result<(), TaskCompletionError> {
        let Some(task) = self.tasks.get(&self.task()) else {
            return Err(TaskCompletionError::NoCurrentTask);
        };
        if !self.task_finished() {
            return Err(TaskCompletionError::TaskNotFullfilled);
        }
        if self
            .inventory()
            .has_room_for_multiple(&task.rewards().items)
        {
            return Err(TaskCompletionError::InsufficientInventorySpace);
        }
        if !self
            .current_map()
            .content_type_is(MapContentType::TasksMaster)
            || !self
                .current_map()
                .content_code_is(&task.r#type().to_string())
        {
            return Err(TaskCompletionError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn exchange_tasks_coins(&self) -> Result<RewardsSchema, TasksCoinExchangeError> {
        self.can_exchange_tasks_coins()?;
        Ok(self.inner.request_exchange_tasks_coin()?)
    }

    pub fn can_exchange_tasks_coins(&self) -> Result<(), TasksCoinExchangeError> {
        let coins_in_inv = self.inventory().total_of(TASKS_COIN);
        if coins_in_inv < TASK_EXCHANGE_PRICE {
            return Err(TasksCoinExchangeError::InsufficientTasksCoinQuantity);
        }
        let extra_quantity = self
            .tasks
            .rewards
            .max_quantity()
            .saturating_sub(TASK_EXCHANGE_PRICE);
        if self.inventory().free_space() < extra_quantity
            || self.inventory().free_slots() < 1 && coins_in_inv > TASK_EXCHANGE_PRICE
        {
            return Err(TasksCoinExchangeError::InsufficientInventorySpace);
        }
        if !self
            .current_map()
            .content_type_is(MapContentType::TasksMaster)
        {
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
        Ok(self.inner.request_npc_buy(item_code, quantity)?)
    }

    fn can_npc_buy(&self, item_code: &str, quantity: u32) -> Result<(), BuyNpcError> {
        if self.items.get(item_code).is_none() {
            return Err(BuyNpcError::ItemNotFound);
        };
        let Some(item) = self.npcs.items.get(item_code) else {
            return Err(BuyNpcError::ItemNotBuyable);
        };
        let Some(buy_price) = item.buy_price() else {
            return Err(BuyNpcError::ItemNotBuyable);
        };
        if item.currency() == GOLD {
            if self.gold() < buy_price * quantity {
                return Err(BuyNpcError::InsufficientGold);
            }
        } else if self.inventory().total_of(item.currency()) < buy_price * quantity {
            return Err(BuyNpcError::InsufficientQuantity);
        }
        Ok(())
    }

    pub fn npc_sell(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, SellNpcError> {
        self.can_npc_sell(item_code, quantity)?;
        Ok(self.inner.request_npc_sell(item_code, quantity)?)
    }

    fn can_npc_sell(&self, item_code: &str, quantity: u32) -> Result<(), SellNpcError> {
        if self.items.get(item_code).is_none() {
            return Err(SellNpcError::ItemNotFound);
        };
        if !self.items.is_salable(item_code) {
            return Err(SellNpcError::ItemNotSalable);
        }
        if self.inventory().total_of(item_code) < quantity {
            return Err(SellNpcError::InsufficientQuantity);
        }
        Ok(())
    }

    pub fn give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), GiveItemError> {
        self.can_give_item(items, character)?;
        Ok(self.inner.request_give_item(items, character)?)
    }

    pub fn can_give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), GiveItemError> {
        for item in items.iter() {
            if self.items.get(&item.code).is_none() {
                return Err(GiveItemError::ItemNotFound);
            }
            if self.inventory().total_of(&item.code) < item.quantity {
                return Err(GiveItemError::InsufficientQuantity);
            }
        }
        let Some(character) = self.account.get_character_by_name(character) else {
            return Err(GiveItemError::CharacterNotFound);
        };
        if character.position() != self.position() {
            return Err(GiveItemError::CharacterNotFound);
        }
        if !character.inventory().has_room_for_multiple(items) {
            return Err(GiveItemError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn give_gold(&self, quantity: u32, character: &str) -> Result<(), GiveGoldError> {
        self.can_give_gold(quantity, character)?;
        Ok(self.inner.request_give_gold(quantity, character)?)
    }

    pub fn can_give_gold(&self, quantity: u32, character: &str) -> Result<(), GiveGoldError> {
        if self.gold() < quantity {
            return Err(GiveGoldError::InsufficientGold);
        }
        let Some(character) = self.account.get_character_by_name(character) else {
            return Err(GiveGoldError::CharacterNotFound);
        };
        if character.position() != self.position() {
            return Err(GiveGoldError::CharacterNotFound);
        }
        Ok(())
    }

    pub fn ge_buy_order(
        &self,
        id: &str,
        quantity: u32,
    ) -> Result<GeTransactionSchema, GeBuyOrderError> {
        self.can_ge_buy_order(id, quantity)?;
        Ok(self.inner.request_ge_buy_order(id, quantity)?)
    }

    pub fn can_ge_buy_order(&self, id: &str, quantity: u32) -> Result<(), GeBuyOrderError> {
        let Some(order) = self.grand_exchange.get_order_by_id(id) else {
            return Err(GeBuyOrderError::OrderNotFound);
        };
        if self.account.name == order.seller {
            return Err(GeBuyOrderError::CannotTradeWithSelf);
        }
        if order.quantity < quantity {
            return Err(GeBuyOrderError::InsufficientQuantity);
        }
        if self.gold() < order.price * quantity {
            return Err(GeBuyOrderError::InsufficientGold);
        }
        if !self.inventory().has_room_for(&order.code, quantity) {
            return Err(GeBuyOrderError::InsufficientInventorySpace);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
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
            .inner
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
        if !item.is_tradable() {
            return Err(GeCreateOrderError::ItemNotSalable);
        }
        if self.inventory().total_of(item.code()) < quantity {
            return Err(GeCreateOrderError::InsufficientQuantity);
        }
        if !self.gold() < ((price * quantity) as f32 * 0.03) as u32 {
            return Err(GeCreateOrderError::InsufficientGold);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(GeCreateOrderError::NoGrandExchangeOnMap);
        }
        Ok(())
    }

    pub fn ge_cancel_order(&self, id: &str) -> Result<GeTransactionSchema, GeCancelOrderError> {
        self.can_ge_cancel_order(id)?;
        Ok(self.inner.request_ge_cancel_order(id)?)
    }

    pub fn can_ge_cancel_order(&self, id: &str) -> Result<(), GeCancelOrderError> {
        let Some(order) = self.grand_exchange.get_order_by_id(id) else {
            return Err(GeCancelOrderError::OrderNotFound);
        };
        if self.account.name != order.seller {
            return Err(GeCancelOrderError::OrderNotOwned);
        }
        if !self.inventory().has_room_for(&order.code, order.quantity) {
            return Err(GeCancelOrderError::InsufficientInventorySpace);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(GeCancelOrderError::NoGrandExchangeOnMap);
        }
        Ok(())
    }

    pub fn gear(&self) -> Gear {
        let d = self.data();
        Gear {
            weapon: self.items.get(&d.weapon_slot),
            shield: self.items.get(&d.shield_slot),
            helmet: self.items.get(&d.helmet_slot),
            body_armor: self.items.get(&d.body_armor_slot),
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
            rune: self.items.get(&d.rune_slot),
            bag: self.items.get(&d.bag_slot),
        }
    }

    // TODO: return a result
    pub fn meets_conditions_for(&self, entity: &impl HasConditions) -> bool {
        entity.conditions().iter().flatten().all(|condition| {
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
                    .account()
                    .get_achievement(&condition.code)
                    .is_some_and(|a| a.completed_at.is_some()),
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

    pub fn account(&self) -> Arc<AccountClient> {
        self.account.clone()
    }

    pub fn remaining_cooldown(&self) -> Duration {
        self.inner.remaining_cooldown()
    }

    pub fn current_map(&self) -> Map {
        let (layer, x, y) = self.position();
        self.maps.get(layer, x, y).unwrap()
    }
}

impl HasCharacterData for CharacterClient {
    fn data(&self) -> Arc<CharacterSchema> {
        self.inner.data()
    }

    fn refresh_data(&self) {
        self.inner.refresh_data();
    }

    fn update_data(&self, schema: CharacterSchema) {
        self.inner.update_data(schema);
    }
}

pub trait HasCharacterData {
    fn data(&self) -> Arc<CharacterSchema>;
    fn refresh_data(&self);
    fn update_data(&self, schema: CharacterSchema);

    fn inventory(&self) -> InventoryClient {
        InventoryClient::new(self.data())
    }

    fn name(&self) -> String {
        self.data().name.to_owned()
    }

    /// Returns the `Character` position (coordinates).
    fn position(&self) -> (MapLayer, i32, i32) {
        let d = self.data();
        (d.layer, d.x, d.y)
    }

    fn level(&self) -> u32 {
        self.data().level as u32
    }

    /// Returns the `Character` level in the given `skill`.
    fn skill_level(&self, skill: Skill) -> u32 {
        let d = self.data();
        (match skill {
            Skill::Combat => d.level,
            Skill::Mining => d.mining_level,
            Skill::Woodcutting => d.woodcutting_level,
            Skill::Fishing => d.fishing_level,
            Skill::Weaponcrafting => d.weaponcrafting_level,
            Skill::Gearcrafting => d.gearcrafting_level,
            Skill::Jewelrycrafting => d.jewelrycrafting_level,
            Skill::Cooking => d.cooking_level,
            Skill::Alchemy => d.alchemy_level,
        }) as u32
    }

    fn skill_xp(&self, skill: Skill) -> i32 {
        let d = self.data();
        match skill {
            Skill::Combat => d.xp,
            Skill::Mining => d.mining_xp,
            Skill::Woodcutting => d.woodcutting_xp,
            Skill::Fishing => d.fishing_xp,
            Skill::Weaponcrafting => d.weaponcrafting_xp,
            Skill::Gearcrafting => d.gearcrafting_xp,
            Skill::Jewelrycrafting => d.jewelrycrafting_xp,
            Skill::Cooking => d.cooking_xp,
            Skill::Alchemy => d.alchemy_xp,
        }
    }

    fn skill_max_xp(&self, skill: Skill) -> i32 {
        let d = self.data();
        match skill {
            Skill::Combat => d.max_xp,
            Skill::Mining => d.mining_max_xp,
            Skill::Woodcutting => d.woodcutting_max_xp,
            Skill::Fishing => d.fishing_max_xp,
            Skill::Weaponcrafting => d.weaponcrafting_max_xp,
            Skill::Gearcrafting => d.gearcrafting_max_xp,
            Skill::Jewelrycrafting => d.jewelrycrafting_max_xp,
            Skill::Cooking => d.cooking_max_xp,
            Skill::Alchemy => d.alchemy_max_xp,
        }
    }

    fn max_health(&self) -> i32 {
        self.data().max_hp
    }

    fn health(&self) -> i32 {
        self.data().hp
    }

    fn missing_hp(&self) -> i32 {
        self.max_health() - self.health()
    }

    fn gold(&self) -> u32 {
        self.data().gold as u32
    }

    fn quantity_in_slot(&self, slot: Slot) -> u32 {
        match slot {
            Slot::Utility1 => self.data().utility1_slot_quantity,
            Slot::Utility2 => self.data().utility2_slot_quantity,
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
            | Slot::Artifact3
            | Slot::Bag
            | Slot::Rune => {
                if self.equiped_in(slot).is_empty() {
                    0
                } else {
                    1
                }
            }
        }
    }

    fn task(&self) -> String {
        self.data().task.to_owned()
    }

    fn task_type(&self) -> Option<TaskType> {
        match self.data().task_type.as_str() {
            "monsters" => Some(TaskType::Monsters),
            "items" => Some(TaskType::Items),
            _ => None,
        }
    }

    fn task_progress(&self) -> u32 {
        self.data().task_progress as u32
    }

    fn task_total(&self) -> u32 {
        self.data().task_total as u32
    }

    fn task_missing(&self) -> u32 {
        self.task_total().saturating_sub(self.task_progress())
    }

    fn task_finished(&self) -> bool {
        !self.task().is_empty() && self.task_missing() < 1
    }

    /// Returns the cooldown expiration timestamp of the `Character`.
    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.data()
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }

    fn equiped_in(&self, slot: Slot) -> String {
        let d = self.data();
        match slot {
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
            Slot::Bag => &d.bag_slot,
            Slot::Rune => &d.rune_slot,
        }
        .clone()
    }

    fn has_equiped(&self, item_code: &str) -> u32 {
        Slot::iter()
            .filter_map(|s| (self.equiped_in(s) == item_code).then_some(self.quantity_in_slot(s)))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapi::models::InventorySlot;
    use std::sync::RwLock;

    impl From<CharacterSchema> for CharacterClient {
        fn from(value: CharacterSchema) -> Self {
            Self::new(
                1,
                Arc::new(RwLock::new(Arc::new(value))),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            )
        }
    }

    //TODO: fix test
    // #[test]
    // fn can_fight() {
    //     // monster on 0,2 is "cow"
    //     let char = BaseCharacter::from(CharacterSchema {
    //         x: 0,
    //         y: 2,
    //         inventory_max_items: 100,
    //         ..Default::default()
    //     });
    //     assert!(char.can_fight().is_ok());
    //     let char = BaseCharacter::from(CharacterSchema {
    //         x: 0,
    //         y: 2,
    //         inventory_max_items: &char.map().monster().unwrap().max_drop_quantity() - 1,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_fight(),
    //         Err(FightError::InsufficientInventorySpace)
    //     ));
    // }

    //TODO: fix test
    // #[test]
    // fn can_gather() {
    //     let char = BaseCharacter::from(CharacterSchema {
    //         x: 2,
    //         y: 0,
    //         mining_level: 1,
    //         inventory_max_items: 100,
    //         ..Default::default()
    //     });
    //     assert!(char.can_gather().is_ok());
    //     let char = BaseCharacter::from(CharacterSchema {
    //         x: 0,
    //         y: 0,
    //         mining_level: 1,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_gather(),
    //         Err(GatherError::NoResourceOnMap)
    //     ));
    //     let char = BaseCharacter::from(CharacterSchema {
    //         x: 1,
    //         y: 7,
    //         mining_level: 1,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_gather(),
    //         Err(GatherError::SkillLevelInsufficient)
    //     ));
    //     let char = BaseCharacter::from(CharacterSchema {
    //         x: 2,
    //         y: 0,
    //         mining_level: 1,
    //         inventory_max_items: char
    //             .maps
    //             .get(2, 0)
    //             .unwrap()
    //             .resource()
    //             .unwrap()
    //             .max_drop_quantity()
    //             - 1,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_gather(),
    //         Err(GatherError::InsufficientInventorySpace)
    //     ));
    // }

    #[test]
    fn can_move() {
        let char = CharacterClient::from(CharacterSchema::default());
        assert!(char.can_move(0, 0).is_ok());
        assert!(matches!(
            char.can_move(1000, 0),
            Err(MoveError::MapNotFound)
        ));
    }

    #[test]
    fn can_use() {
        let item1 = "cooked_chicken";
        let item2 = "cooked_shrimp";
        let char = CharacterClient::from(CharacterSchema {
            level: 5,
            inventory: Some(vec![
                InventorySlot {
                    slot: 0,
                    code: item1.to_owned(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 1,
                    code: item2.to_owned(),
                    quantity: 1,
                },
            ]),
            ..Default::default()
        });
        assert!(matches!(
            char.can_use_item("random_item", 1),
            Err(UseError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_use_item("copper", 1),
            Err(UseError::ItemNotConsumable)
        ));
        assert!(matches!(
            char.can_use_item(item1, 5),
            Err(UseError::InsufficientQuantity)
        ));
        assert!(matches!(
            char.can_use_item(item2, 1),
            Err(UseError::InsufficientCharacterLevel)
        ));
        assert!(char.can_use_item(item1, 1).is_ok());
    }

    #[test]
    fn can_craft() {
        let char = CharacterClient::from(CharacterSchema {
            cooking_level: 1,
            inventory: Some(vec![
                InventorySlot {
                    slot: 0,
                    code: "gudgeon".to_string(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 1,
                    code: "shrimp".to_string(),
                    quantity: 1,
                },
            ]),
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(matches!(
            char.can_craft("random_item", 1),
            Err(CraftError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_craft("copper_ore", 1),
            Err(CraftError::ItemNotCraftable)
        ));
        assert!(matches!(
            char.can_craft("cooked_chicken", 1),
            Err(CraftError::InsufficientMaterials)
        ));
        assert!(matches!(
            char.can_craft("cooked_gudgeon", 5),
            Err(CraftError::InsufficientMaterials)
        ));
        assert!(matches!(
            char.can_craft("cooked_shrimp", 1),
            Err(CraftError::InsufficientSkillLevel)
        ));
        assert!(matches!(
            char.can_craft("cooked_gudgeon", 1),
            Err(CraftError::NoWorkshopOnMap)
        ));
        let char = CharacterClient::from(CharacterSchema {
            cooking_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "gudgeon".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 100,
            x: 1,
            y: 1,
            ..Default::default()
        });
        assert!(char.can_craft("cooked_gudgeon", 1).is_ok());
    }

    #[test]
    fn can_recycle() {
        let char = CharacterClient::from(CharacterSchema {
            cooking_level: 1,
            weaponcrafting_level: 1,
            inventory: Some(vec![
                InventorySlot {
                    slot: 0,
                    code: "copper_dagger".to_string(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 1,
                    code: "iron_sword".to_string(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 2,
                    code: "cooked_gudgeon".to_string(),
                    quantity: 1,
                },
            ]),
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(matches!(
            char.can_recycle("random_item", 1),
            Err(RecycleError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_recycle("cooked_gudgeon", 1),
            Err(RecycleError::ItemNotRecyclable)
        ));
        assert!(matches!(
            char.can_recycle("wooden_staff", 1),
            Err(RecycleError::InsufficientQuantity)
        ));
        assert!(matches!(
            char.can_recycle("iron_sword", 1),
            Err(RecycleError::InsufficientSkillLevel)
        ));
        assert!(matches!(
            char.can_recycle("copper_dagger", 1),
            Err(RecycleError::NoWorkshopOnMap)
        ));
        let char = CharacterClient::from(CharacterSchema {
            weaponcrafting_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "copper_dagger".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 1,
            x: 2,
            y: 1,
            ..Default::default()
        });
        assert!(matches!(
            char.can_recycle("copper_dagger", 1),
            Err(RecycleError::InsufficientInventorySpace)
        ));
        let char = CharacterClient::from(CharacterSchema {
            weaponcrafting_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "copper_dagger".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 100,
            x: 2,
            y: 1,
            ..Default::default()
        });
        assert!(char.can_recycle("copper_dagger", 1).is_ok());
    }

    #[test]
    fn can_delete() {
        let char = CharacterClient::from(CharacterSchema {
            cooking_level: 1,
            weaponcrafting_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "copper_dagger".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(matches!(
            char.can_delete("random_item", 1),
            Err(DeleteError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_delete("copper_dagger", 2),
            Err(DeleteError::InsufficientQuantity)
        ));
        assert!(char.can_delete("copper_dagger", 1).is_ok());
    }

    #[test]
    fn can_withdraw() {
        let char = CharacterClient::from(CharacterSchema {
            inventory_max_items: 100,
            ..Default::default()
        });
        char.bank.update_content(vec![
            SimpleItemSchema {
                code: "copper_dagger".to_string(),
                quantity: 1,
            },
            SimpleItemSchema {
                code: "iron_sword".to_string(),
                quantity: 101,
            },
        ]);
        // TODO: rewrite these tests
        // assert!(matches!(
        //     char.can_withdraw_items("random_item", 1),
        //     Err(WithdrawError::ItemNotFound)
        // ));
        // assert!(matches!(
        //     char.can_withdraw_item("copper_dagger", 2),
        //     Err(WithdrawError::InsufficientQuantity)
        // ));
        // assert!(matches!(
        //     char.can_withdraw_item("iron_sword", 101),
        //     Err(WithdrawError::InsufficientInventorySpace)
        // ));
        // assert!(matches!(
        //     char.can_withdraw_item("iron_sword", 10),
        //     Err(WithdrawError::NoBankOnMap)
        // ));
        let char = CharacterClient::from(CharacterSchema {
            inventory_max_items: 100,
            x: 4,
            y: 1,
            ..Default::default()
        });
        char.bank.update_content(vec![
            SimpleItemSchema {
                code: "copper_dagger".to_string(),
                quantity: 1,
            },
            SimpleItemSchema {
                code: "iron_sword".to_string(),
                quantity: 101,
            },
        ]);
        // assert!(char.can_withdraw_item("iron_sword", 10).is_ok());
    }
    //TODO: add more tests
}
