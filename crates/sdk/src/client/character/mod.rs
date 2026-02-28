use crate::{
    AccountClient, Code, CollectionClient, GOLD, Gear, HasConditions, ItemContainer, Level,
    LimitedContainer, SlotLimited, SpaceLimited, TASK_EXCHANGE_PRICE, TASKS_COIN, TasksClient,
    character::{
        action::{CharacterAction, MoveCharacter},
        data_handle::CharacterDataHandle,
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
    entities::{CharacterTrait, Map, RawCharacter},
    gear::Slot,
    grand_exchange::GrandExchangeClient,
    simulator::HasEffects,
    skill::Skill,
};
use api::ArtifactApi;
use chrono::{DateTime, Utc};
use openapi::models::{
    CharacterFightSchema, CharacterSchema, ConditionOperator, GeTransactionSchema, InventorySlot,
    MapContentType, MapLayer, NpcItemTransactionSchema, RecyclingItemsSchema, RewardsSchema,
    SimpleItemSchema, SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, TaskType,
};
use std::{str::FromStr, sync::Arc, time::Duration};

pub use inventory::InventoryClient;

mod request_handler;

pub mod action;
pub mod action_request;
pub mod data_handle;
pub mod error;
pub mod inventory;
pub mod responses;

#[derive(Default, Debug, Clone)]
pub struct CharacterClient(Arc<CharacterClientInner>);

#[derive(Default, Debug)]
pub struct CharacterClientInner {
    pub id: usize,
    name: String,
    handler: CharacterRequestHandler,
    account: AccountClient,
    bank: BankClient,
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
        data: CharacterDataHandle,
        account: AccountClient,
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
        Self(Arc::new(CharacterClientInner {
            id,
            name: data.read().name().to_string(),
            handler: CharacterRequestHandler::new(api, data, account.clone(), server.clone()),
            bank: account.bank(),
            account,
            items,
            resources,
            monsters,
            maps,
            npcs,
            tasks,
            grand_exchange,
        }))
    }

    pub fn id(&self) -> usize {
        self.0.id
    }

    pub(crate) fn handler(&self) -> &CharacterRequestHandler {
        &self.0.handler
    }

    pub fn inventory(&self) -> InventoryClient {
        todo!()
    }

    pub fn r#move(&self, x: i32, y: i32) -> Result<Map, MoveError> {
        MoveCharacter {
            x,
            y,
            maps: self.0.maps.clone(),
        }
        .execute(self)
    }

    pub fn transition(&self) -> Result<Map, TransitionError> {
        self.can_transition()?;
        Ok(self.0.handler.request_transition()?)
    }

    pub fn can_transition(&self) -> Result<(), TransitionError> {
        let map = self.current_map();
        let Some(ref transition) = map.interactions().transition else {
            return Err(TransitionError::TransitionNotFound);
        };
        if !self.meets_conditions_for(&transition.as_ref()) {
            return Err(TransitionError::ConditionsNotMet);
        }
        Ok(())
    }

    pub fn fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, FightError> {
        self.can_fight(participants)?;
        Ok(self.0.handler.request_fight(participants)?)
    }

    pub fn can_fight(&self, participants: Option<&[String; 2]>) -> Result<(), FightError> {
        let Some(monster) = self
            .current_map()
            .content_code()
            .and_then(|code| self.0.monsters.get(code))
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
            let Some(char) = self.0.account.get_character_by_name(name) else {
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
        Ok(self.0.handler.request_gather()?)
    }

    pub fn can_gather(&self) -> Result<(), GatherError> {
        let Some(resource) = self
            .current_map()
            .content_code()
            .and_then(|code| self.0.resources.get(code))
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
            return Ok(self.0.handler.request_rest()?);
        }
        Ok(0)
    }

    pub fn craft(&self, item_code: &str, quantity: u32) -> Result<SkillInfoSchema, CraftError> {
        self.can_craft(item_code, quantity)?;
        Ok(self.0.handler.request_craft(item_code, quantity)?)
    }

    pub fn can_craft(&self, item_code: &str, quantity: u32) -> Result<(), CraftError> {
        let Some(item) = self.0.items.get(item_code) else {
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
        Ok(self.0.handler.request_recycle(item_code, quantity)?)
    }

    pub fn can_recycle(&self, item_code: &str, quantity: u32) -> Result<(), RecycleError> {
        let Some(item) = self.0.items.get(item_code) else {
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
        Ok(self.0.handler.request_delete(item_code, quantity)?)
    }

    pub fn can_delete(&self, item_code: &str, quantity: u32) -> Result<(), DeleteError> {
        if self.0.items.get(item_code).is_none() {
            return Err(DeleteError::ItemNotFound);
        };
        if self.inventory().total_of(item_code) < quantity {
            return Err(DeleteError::InsufficientQuantity);
        }
        Ok(())
    }

    pub fn deposit_item(&self, items: &[SimpleItemSchema]) -> Result<(), DepositError> {
        self.can_deposit_items(items)?;
        Ok(self.0.handler.request_deposit_item(items)?)
    }

    pub fn can_deposit_items(&self, items: &[SimpleItemSchema]) -> Result<(), DepositError> {
        for item in items.iter() {
            if self.0.items.get(&item.code).is_none() {
                return Err(DepositError::ItemNotFound);
            };
            if self.inventory().total_of(&item.code) < item.quantity {
                return Err(DepositError::InsufficientQuantity);
            }
        }
        if !self.0.bank.has_room_for_multiple(items) {
            return Err(DepositError::InsufficientBankSpace);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(DepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), WithdrawError> {
        self.can_withdraw_items(items)?;
        Ok(self.0.handler.request_withdraw_item(items)?)
    }

    pub fn can_withdraw_items(&self, items: &[SimpleItemSchema]) -> Result<(), WithdrawError> {
        if items
            .iter()
            .any(|i| self.0.bank.total_of(&i.code) < i.quantity)
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
        Ok(self.0.handler.request_deposit_gold(quantity)?)
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
        Ok(self.0.handler.request_withdraw_gold(quantity)?)
    }

    pub fn can_withdraw_gold(&self, quantity: u32) -> Result<(), GoldWithdrawError> {
        if self.0.bank.gold() < quantity {
            return Err(GoldWithdrawError::InsufficientGold);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(GoldWithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn expand_bank(&self) -> Result<u32, BankExpansionError> {
        self.can_expand_bank()?;
        Ok(self.0.handler.request_expand_bank()?)
    }

    pub fn can_expand_bank(&self) -> Result<(), BankExpansionError> {
        if self.gold() < self.0.bank.next_expansion_cost() {
            return Err(BankExpansionError::InsufficientGold);
        }
        if !self.current_map().content_type_is(MapContentType::Bank) {
            return Err(BankExpansionError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn equip(&self, item_code: &str, slot: Slot, quantity: u32) -> Result<(), EquipError> {
        self.can_equip(item_code, slot, quantity)?;
        Ok(self.0.handler.request_equip(item_code, slot, quantity)?)
    }

    pub fn can_equip(&self, item_code: &str, slot: Slot, quantity: u32) -> Result<(), EquipError> {
        let Some(item) = self.0.items.get(item_code) else {
            return Err(EquipError::ItemNotFound);
        };
        if self.inventory().total_of(item_code) < quantity {
            return Err(EquipError::InsufficientQuantity);
        }
        if let Some(equiped) = self.0.items.get(&self.equiped_in(slot)) {
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
        Ok(self.0.handler.request_unequip(slot, quantity)?)
    }

    pub fn can_unequip(&self, slot: Slot, quantity: u32) -> Result<(), UnequipError> {
        let Some(equiped) = self.0.items.get(&self.equiped_in(slot)) else {
            return Err(UnequipError::SlotEmpty);
        };
        if self.hp() <= equiped.health() {
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
        Ok(self.0.handler.request_use_item(item_code, quantity)?)
    }

    pub fn can_use_item(&self, item_code: &str, quantity: u32) -> Result<(), UseError> {
        let Some(item) = self.0.items.get(item_code) else {
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
        Ok(self.0.handler.request_accept_task()?)
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
        Ok(self.0.handler.request_cancel_task()?)
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
            .0
            .handler
            .request_trade_task_item(item_code, quantity)?)
    }

    pub fn can_trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<(), TaskTradeError> {
        if self.0.items.get(item_code).is_none() {
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
        Ok(self.0.handler.request_complete_task()?)
    }

    pub fn can_complete_task(&self) -> Result<(), TaskCompletionError> {
        let Some(task) = self.0.tasks.get(&self.task()) else {
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
        Ok(self.0.handler.request_exchange_tasks_coin()?)
    }

    pub fn can_exchange_tasks_coins(&self) -> Result<(), TasksCoinExchangeError> {
        let coins_in_inv = self.inventory().total_of(TASKS_COIN);
        if coins_in_inv < TASK_EXCHANGE_PRICE {
            return Err(TasksCoinExchangeError::InsufficientTasksCoinQuantity);
        }
        let extra_quantity = self
            .0
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
        Ok(self.0.handler.request_npc_buy(item_code, quantity)?)
    }

    fn can_npc_buy(&self, item_code: &str, quantity: u32) -> Result<(), BuyNpcError> {
        if self.0.items.get(item_code).is_none() {
            return Err(BuyNpcError::ItemNotFound);
        };
        let Some(item) = self.0.npcs.items.get(item_code) else {
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
        Ok(self.0.handler.request_npc_sell(item_code, quantity)?)
    }

    fn can_npc_sell(&self, item_code: &str, quantity: u32) -> Result<(), SellNpcError> {
        if self.0.items.get(item_code).is_none() {
            return Err(SellNpcError::ItemNotFound);
        };
        if !self.0.items.is_salable(item_code) {
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
        Ok(self.0.handler.request_give_item(items, character)?)
    }

    pub fn can_give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), GiveItemError> {
        for item in items.iter() {
            if self.0.items.get(&item.code).is_none() {
                return Err(GiveItemError::ItemNotFound);
            }
            if self.inventory().total_of(&item.code) < item.quantity {
                return Err(GiveItemError::InsufficientQuantity);
            }
        }
        let Some(character) = self.0.account.get_character_by_name(character) else {
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
        Ok(self.0.handler.request_give_gold(quantity, character)?)
    }

    pub fn can_give_gold(&self, quantity: u32, character: &str) -> Result<(), GiveGoldError> {
        if self.gold() < quantity {
            return Err(GiveGoldError::InsufficientGold);
        }
        let Some(character) = self.0.account.get_character_by_name(character) else {
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
        Ok(self.0.handler.request_ge_buy_order(id, quantity)?)
    }

    pub fn can_ge_buy_order(&self, id: &str, quantity: u32) -> Result<(), GeBuyOrderError> {
        let Some(order) = self.0.grand_exchange.get_order_by_id(id) else {
            return Err(GeBuyOrderError::OrderNotFound);
        };
        if self.0.account.name() == order.seller {
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
            .0
            .handler
            .request_ge_create_order(item_code, quantity, price)?)
    }

    pub fn can_ge_create_order(
        &self,
        item_code: &str,
        quantity: u32,
        price: u32,
    ) -> Result<(), GeCreateOrderError> {
        let Some(item) = self.0.items.get(item_code) else {
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
        Ok(self.0.handler.request_ge_cancel_order(id)?)
    }

    pub fn can_ge_cancel_order(&self, id: &str) -> Result<(), GeCancelOrderError> {
        let Some(order) = self.0.grand_exchange.get_order_by_id(id) else {
            return Err(GeCancelOrderError::OrderNotFound);
        };
        if self.0.account.name() != order.seller {
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
        Gear {
            weapon: self.0.items.get(&self.equiped_in(Slot::Weapon)),
            shield: self.0.items.get(&self.equiped_in(Slot::Shield)),
            helmet: self.0.items.get(&self.equiped_in(Slot::Helmet)),
            body_armor: self.0.items.get(&self.equiped_in(Slot::BodyArmor)),
            leg_armor: self.0.items.get(&self.equiped_in(Slot::LegArmor)),
            boots: self.0.items.get(&self.equiped_in(Slot::Boots)),
            ring1: self.0.items.get(&self.equiped_in(Slot::Ring1)),
            ring2: self.0.items.get(&self.equiped_in(Slot::Ring2)),
            amulet: self.0.items.get(&self.equiped_in(Slot::Amulet)),
            artifact1: self.0.items.get(&self.equiped_in(Slot::Artifact1)),
            artifact2: self.0.items.get(&self.equiped_in(Slot::Artifact2)),
            artifact3: self.0.items.get(&self.equiped_in(Slot::Artifact3)),
            utility1: self.0.items.get(&self.equiped_in(Slot::Utility1)),
            utility2: self.0.items.get(&self.equiped_in(Slot::Utility1)),
            rune: self.0.items.get(&self.equiped_in(Slot::Rune)),
            bag: self.0.items.get(&self.equiped_in(Slot::Bag)),
        }
    }

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
                    .0
                    .account
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

    pub fn account(&self) -> AccountClient {
        self.0.account.clone()
    }

    pub fn remaining_cooldown(&self) -> Duration {
        self.0.handler.remaining_cooldown()
    }

    pub fn current_map(&self) -> Map {
        self.0.maps.get(self.position()).unwrap()
    }
}

pub trait HandleCharacterData: CharacterTrait {
    fn data(&self) -> RawCharacter;
    fn refresh_data(&self);
    fn update_data(&self, schema: CharacterSchema);
}

impl HandleCharacterData for CharacterClient {
    fn data(&self) -> RawCharacter {
        self.0.handler.data()
    }

    fn refresh_data(&self) {
        self.0.handler.refresh_data()
    }

    fn update_data(&self, schema: CharacterSchema) {
        self.0.handler.update_data(schema);
    }
}

pub trait MeetsConditionsFor: CharacterTrait {
    fn account(&self) -> AccountClient;

    fn inventory(&self) -> InventoryClient;
}

impl CharacterTrait for CharacterClient {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn position(&self) -> (MapLayer, i32, i32) {
        self.data().position()
    }

    fn skill_level(&self, skill: Skill) -> u32 {
        self.data().skill_level(skill)
    }

    fn skill_xp(&self, skill: Skill) -> i32 {
        self.data().skill_xp(skill)
    }

    fn skill_max_xp(&self, skill: Skill) -> i32 {
        self.data().skill_max_xp(skill)
    }

    fn hp(&self) -> i32 {
        self.data().hp()
    }

    fn max_hp(&self) -> i32 {
        self.data().max_hp()
    }

    fn missing_hp(&self) -> i32 {
        self.data().missing_hp()
    }

    fn task(&self) -> String {
        self.data().task()
    }

    fn task_type(&self) -> Option<TaskType> {
        self.data().task_type()
    }

    fn task_progress(&self) -> u32 {
        self.data().task_progress()
    }

    fn task_total(&self) -> u32 {
        self.data().task_total()
    }

    fn task_missing(&self) -> u32 {
        self.data().task_missing()
    }

    fn task_finished(&self) -> bool {
        self.data().task_finished()
    }

    fn inventory_items(&self) -> Option<Vec<InventorySlot>> {
        self.data().inventory_items()
    }

    fn inventory_max_items(&self) -> i32 {
        self.data().inventory_max_items()
    }

    fn gold(&self) -> u32 {
        self.data().gold()
    }

    fn equiped_in(&self, slot: Slot) -> String {
        self.data().equiped_in(slot)
    }

    fn has_equiped(&self, item_code: &str) -> u32 {
        self.data().has_equiped(item_code)
    }

    fn quantity_in_slot(&self, slot: Slot) -> u32 {
        self.data().quantity_in_slot(slot)
    }

    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.data().cooldown_expiration()
    }
}

impl Level for CharacterClient {
    fn level(&self) -> u32 {
        self.data().level()
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use openapi::models::InventorySlot;

    // impl From<CharacterSchema> for CharacterClient {
    //     fn from(value: CharacterSchema) -> Self {
    //         Self::new(
    //             1,
    //             Arc::new(RwLock::new(Arc::new(value))),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //             Default::default(),
    //         )
    //     }
    // }

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
    //             .0.maps
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

    // #[test]
    // fn can_move() {
    //     let char = CharacterClient::from(CharacterSchema::default());
    //     assert!(char.can_move(0, 0).is_ok());
    //     assert!(matches!(
    //         char.can_move(1000, 0),
    //         Err(MoveError::MapNotFound)
    //     ));
    // }

    // #[test]
    // fn can_use() {
    //     let item1 = "cooked_chicken";
    //     let item2 = "cooked_shrimp";
    //     let char = CharacterClient::from(CharacterSchema {
    //         level: 5,
    //         inventory: Some(vec![
    //             InventorySlot {
    //                 slot: 0,
    //                 code: item1.to_owned(),
    //                 quantity: 1,
    //             },
    //             InventorySlot {
    //                 slot: 1,
    //                 code: item2.to_owned(),
    //                 quantity: 1,
    //             },
    //         ]),
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_use_item("random_item", 1),
    //         Err(UseError::ItemNotFound)
    //     ));
    //     assert!(matches!(
    //         char.can_use_item("copper", 1),
    //         Err(UseError::ItemNotConsumable)
    //     ));
    //     assert!(matches!(
    //         char.can_use_item(item1, 5),
    //         Err(UseError::InsufficientQuantity)
    //     ));
    //     assert!(matches!(
    //         char.can_use_item(item2, 1),
    //         Err(UseError::InsufficientCharacterLevel)
    //     ));
    //     assert!(char.can_use_item(item1, 1).is_ok());
    // }

    // #[test]
    // fn can_craft() {
    //     let char = CharacterClient::from(CharacterSchema {
    //         cooking_level: 1,
    //         inventory: Some(vec![
    //             InventorySlot {
    //                 slot: 0,
    //                 code: "gudgeon".to_string(),
    //                 quantity: 1,
    //             },
    //             InventorySlot {
    //                 slot: 1,
    //                 code: "shrimp".to_string(),
    //                 quantity: 1,
    //             },
    //         ]),
    //         inventory_max_items: 100,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_craft("random_item", 1),
    //         Err(CraftError::ItemNotFound)
    //     ));
    //     assert!(matches!(
    //         char.can_craft("copper_ore", 1),
    //         Err(CraftError::ItemNotCraftable)
    //     ));
    //     assert!(matches!(
    //         char.can_craft("cooked_chicken", 1),
    //         Err(CraftError::InsufficientMaterials)
    //     ));
    //     assert!(matches!(
    //         char.can_craft("cooked_gudgeon", 5),
    //         Err(CraftError::InsufficientMaterials)
    //     ));
    //     assert!(matches!(
    //         char.can_craft("cooked_shrimp", 1),
    //         Err(CraftError::InsufficientSkillLevel)
    //     ));
    //     assert!(matches!(
    //         char.can_craft("cooked_gudgeon", 1),
    //         Err(CraftError::NoWorkshopOnMap)
    //     ));
    //     let char = CharacterClient::from(CharacterSchema {
    //         cooking_level: 1,
    //         inventory: Some(vec![InventorySlot {
    //             slot: 0,
    //             code: "gudgeon".to_string(),
    //             quantity: 1,
    //         }]),
    //         inventory_max_items: 100,
    //         x: 1,
    //         y: 1,
    //         ..Default::default()
    //     });
    //     assert!(char.can_craft("cooked_gudgeon", 1).is_ok());
    // }

    // #[test]
    // fn can_recycle() {
    //     let char = CharacterClient::from(CharacterSchema {
    //         cooking_level: 1,
    //         weaponcrafting_level: 1,
    //         inventory: Some(vec![
    //             InventorySlot {
    //                 slot: 0,
    //                 code: "copper_dagger".to_string(),
    //                 quantity: 1,
    //             },
    //             InventorySlot {
    //                 slot: 1,
    //                 code: "iron_sword".to_string(),
    //                 quantity: 1,
    //             },
    //             InventorySlot {
    //                 slot: 2,
    //                 code: "cooked_gudgeon".to_string(),
    //                 quantity: 1,
    //             },
    //         ]),
    //         inventory_max_items: 100,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_recycle("random_item", 1),
    //         Err(RecycleError::ItemNotFound)
    //     ));
    //     assert!(matches!(
    //         char.can_recycle("cooked_gudgeon", 1),
    //         Err(RecycleError::ItemNotRecyclable)
    //     ));
    //     assert!(matches!(
    //         char.can_recycle("wooden_staff", 1),
    //         Err(RecycleError::InsufficientQuantity)
    //     ));
    //     assert!(matches!(
    //         char.can_recycle("iron_sword", 1),
    //         Err(RecycleError::InsufficientSkillLevel)
    //     ));
    //     assert!(matches!(
    //         char.can_recycle("copper_dagger", 1),
    //         Err(RecycleError::NoWorkshopOnMap)
    //     ));
    //     let char = CharacterClient::from(CharacterSchema {
    //         weaponcrafting_level: 1,
    //         inventory: Some(vec![InventorySlot {
    //             slot: 0,
    //             code: "copper_dagger".to_string(),
    //             quantity: 1,
    //         }]),
    //         inventory_max_items: 1,
    //         x: 2,
    //         y: 1,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_recycle("copper_dagger", 1),
    //         Err(RecycleError::InsufficientInventorySpace)
    //     ));
    //     let char = CharacterClient::from(CharacterSchema {
    //         weaponcrafting_level: 1,
    //         inventory: Some(vec![InventorySlot {
    //             slot: 0,
    //             code: "copper_dagger".to_string(),
    //             quantity: 1,
    //         }]),
    //         inventory_max_items: 100,
    //         x: 2,
    //         y: 1,
    //         ..Default::default()
    //     });
    //     assert!(char.can_recycle("copper_dagger", 1).is_ok());
    // }

    // #[test]
    // fn can_delete() {
    //     let char = CharacterClient::from(CharacterSchema {
    //         cooking_level: 1,
    //         weaponcrafting_level: 1,
    //         inventory: Some(vec![InventorySlot {
    //             slot: 0,
    //             code: "copper_dagger".to_string(),
    //             quantity: 1,
    //         }]),
    //         inventory_max_items: 100,
    //         ..Default::default()
    //     });
    //     assert!(matches!(
    //         char.can_delete("random_item", 1),
    //         Err(DeleteError::ItemNotFound)
    //     ));
    //     assert!(matches!(
    //         char.can_delete("copper_dagger", 2),
    //         Err(DeleteError::InsufficientQuantity)
    //     ));
    //     assert!(char.can_delete("copper_dagger", 1).is_ok());
    // }

    // #[test]
    // fn can_withdraw() {
    //     let char = CharacterClient::from(CharacterSchema {
    //         inventory_max_items: 100,
    //         ..Default::default()
    //     });
    //     char.bank.update_content(vec![
    //         SimpleItemSchema {
    //             code: "copper_dagger".to_string(),
    //             quantity: 1,
    //         },
    //         SimpleItemSchema {
    //             code: "iron_sword".to_string(),
    //             quantity: 101,
    //         },
    //     ]);
    //     // TODO: rewrite these tests
    //     // assert!(matches!(
    //     //     char.can_withdraw_items("random_item", 1),
    //     //     Err(WithdrawError::ItemNotFound)
    //     // ));
    //     // assert!(matches!(
    //     //     char.can_withdraw_item("copper_dagger", 2),
    //     //     Err(WithdrawError::InsufficientQuantity)
    //     // ));
    //     // assert!(matches!(
    //     //     char.can_withdraw_item("iron_sword", 101),
    //     //     Err(WithdrawError::InsufficientInventorySpace)
    //     // ));
    //     // assert!(matches!(
    //     //     char.can_withdraw_item("iron_sword", 10),
    //     //     Err(WithdrawError::NoBankOnMap)
    //     // ));
    //     // let char = CharacterClient::from(CharacterSchema {
    //     //     inventory_max_items: 100,
    //     //     x: 4,
    //     //     y: 1,
    //     //     ..Default::default()
    //     // });
    //     char.bank.update_content(vec![
    //         SimpleItemSchema {
    //             code: "copper_dagger".to_string(),
    //             quantity: 1,
    //         },
    //         SimpleItemSchema {
    //             code: "iron_sword".to_string(),
    //             quantity: 101,
    //         },
    //     ]);
    //     // assert!(char.can_withdraw_item("iron_sword", 10).is_ok());
    // }
    //TODO: add more tests
}
