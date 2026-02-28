use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        my_characters_api::{
            ActionAcceptNewTaskMyNameActionTaskNewPostError,
            ActionBuyBankExpansionMyNameActionBankBuyExpansionPostError,
            ActionCompleteTaskMyNameActionTaskCompletePostError,
            ActionCraftingMyNameActionCraftingPostError,
            ActionDeleteItemMyNameActionDeletePostError,
            ActionDepositBankGoldMyNameActionBankDepositGoldPostError,
            ActionDepositBankItemMyNameActionBankDepositItemPostError,
            ActionEquipItemMyNameActionEquipPostError, ActionFightMyNameActionFightPostError,
            ActionGatheringMyNameActionGatheringPostError,
            ActionGeBuyItemMyNameActionGrandexchangeBuyPostError,
            ActionGeCancelOrderMyNameActionGrandexchangeCancelPostError,
            ActionGeCreateSellOrderMyNameActionGrandexchangeCreateSellOrderPostError,
            ActionGiveGoldMyNameActionGiveGoldPostError,
            ActionGiveItemsMyNameActionGiveItemPostError, ActionMoveMyNameActionMovePostError,
            ActionNpcBuyItemMyNameActionNpcBuyPostError,
            ActionNpcSellItemMyNameActionNpcSellPostError,
            ActionRecyclingMyNameActionRecyclingPostError, ActionRestMyNameActionRestPostError,
            ActionTaskCancelMyNameActionTaskCancelPostError,
            ActionTaskExchangeMyNameActionTaskExchangePostError,
            ActionTaskTradeMyNameActionTaskTradePostError,
            ActionTransitionMyNameActionTransitionPostError,
            ActionUnequipItemMyNameActionUnequipPostError, ActionUseItemMyNameActionUsePostError,
            ActionWithdrawBankGoldMyNameActionBankWithdrawGoldPostError,
            ActionWithdrawBankItemMyNameActionBankWithdrawItemPostError,
            action_accept_new_task_my_name_action_task_new_post,
            action_buy_bank_expansion_my_name_action_bank_buy_expansion_post,
            action_complete_task_my_name_action_task_complete_post,
            action_crafting_my_name_action_crafting_post,
            action_delete_item_my_name_action_delete_post,
            action_deposit_bank_gold_my_name_action_bank_deposit_gold_post,
            action_deposit_bank_item_my_name_action_bank_deposit_item_post,
            action_equip_item_my_name_action_equip_post, action_fight_my_name_action_fight_post,
            action_gathering_my_name_action_gathering_post,
            action_ge_buy_item_my_name_action_grandexchange_buy_post,
            action_ge_cancel_order_my_name_action_grandexchange_cancel_post,
            action_ge_create_sell_order_my_name_action_grandexchange_create_sell_order_post,
            action_give_gold_my_name_action_give_gold_post,
            action_give_items_my_name_action_give_item_post, action_move_my_name_action_move_post,
            action_npc_buy_item_my_name_action_npc_buy_post,
            action_npc_sell_item_my_name_action_npc_sell_post,
            action_recycling_my_name_action_recycling_post, action_rest_my_name_action_rest_post,
            action_task_cancel_my_name_action_task_cancel_post,
            action_task_exchange_my_name_action_task_exchange_post,
            action_task_trade_my_name_action_task_trade_post,
            action_transition_my_name_action_transition_post,
            action_unequip_item_my_name_action_unequip_post,
            action_use_item_my_name_action_use_post,
            action_withdraw_bank_gold_my_name_action_bank_withdraw_gold_post,
            action_withdraw_bank_item_my_name_action_bank_withdraw_item_post,
        },
    },
    models::{
        BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
        BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, CharacterRestResponseSchema,
        CharacterTransitionResponseSchema, CraftingSchema, DeleteItemResponseSchema,
        DepositWithdrawGoldSchema, DestinationSchema, EquipSchema, EquipmentResponseSchema,
        FightRequestSchema, GeBuyOrderSchema, GeCancelOrderSchema,
        GeCreateOrderTransactionResponseSchema, GeOrderCreationrSchema,
        GeTransactionResponseSchema, GiveGoldResponseSchema, GiveGoldSchema,
        GiveItemResponseSchema, GiveItemsSchema, ItemSlot, NpcMerchantBuySchema,
        NpcMerchantTransactionResponseSchema, RecyclingResponseSchema, RecyclingSchema,
        RewardDataResponseSchema, SimpleItemSchema, SkillResponseSchema,
        TaskCancelledResponseSchema, TaskResponseSchema, TaskTradeResponseSchema, UnequipSchema,
        UseItemResponseSchema,
    },
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct MyCharacterApi {
    configuration: Arc<Configuration>,
}

impl MyCharacterApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        MyCharacterApi { configuration }
    }

    pub fn r#move(
        &self,
        name: &str,
        x: i32,
        y: i32,
    ) -> Result<CharacterMovementResponseSchema, Error<ActionMoveMyNameActionMovePostError>> {
        let dest = DestinationSchema {
            x: Some(x),
            y: Some(y),
            map_id: None,
        };
        action_move_my_name_action_move_post(&self.configuration, name, dest)
    }

    pub fn transition(
        &self,
        name: &str,
    ) -> Result<
        CharacterTransitionResponseSchema,
        Error<ActionTransitionMyNameActionTransitionPostError>,
    > {
        action_transition_my_name_action_transition_post(&self.configuration, name)
    }

    pub fn fight(
        &self,
        name: &str,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        let schema = FightRequestSchema {
            participants: participants.map(|p| p.to_vec()),
        };
        action_fight_my_name_action_fight_post(&self.configuration, name, Some(schema))
    }

    pub fn rest(
        &self,
        name: &str,
    ) -> Result<CharacterRestResponseSchema, Error<ActionRestMyNameActionRestPostError>> {
        action_rest_my_name_action_rest_post(&self.configuration, name)
    }

    pub fn gather(
        &self,
        name: &str,
    ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>> {
        action_gathering_my_name_action_gathering_post(&self.configuration, name)
    }

    pub fn craft(
        &self,
        name: &str,
        item_code: &str,
        quantity: u32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        let schema = CraftingSchema {
            code: item_code.to_owned(),
            quantity: Some(quantity),
        };
        action_crafting_my_name_action_crafting_post(&self.configuration, name, schema)
    }

    pub fn recycle(
        &self,
        name: &str,
        item_code: &str,
        quantity: u32,
    ) -> Result<RecyclingResponseSchema, Error<ActionRecyclingMyNameActionRecyclingPostError>> {
        let schema = RecyclingSchema {
            code: item_code.to_owned(),
            quantity: Some(quantity),
        };
        action_recycling_my_name_action_recycling_post(&self.configuration, name, schema)
    }

    pub fn delete(
        &self,
        name: &str,
        item_code: &str,
        quantity: u32,
    ) -> Result<DeleteItemResponseSchema, Error<ActionDeleteItemMyNameActionDeletePostError>> {
        let schema = SimpleItemSchema {
            code: item_code.to_owned(),
            quantity,
        };
        action_delete_item_my_name_action_delete_post(&self.configuration, name, schema)
    }

    pub fn deposit_item(
        &self,
        name: &str,
        items: &[SimpleItemSchema],
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankItemMyNameActionBankDepositItemPostError>,
    > {
        action_deposit_bank_item_my_name_action_bank_deposit_item_post(
            &self.configuration,
            name,
            items.to_vec(),
        )
    }

    pub fn withdraw_item(
        &self,
        name: &str,
        items: &[SimpleItemSchema],
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankItemMyNameActionBankWithdrawItemPostError>,
    > {
        action_withdraw_bank_item_my_name_action_bank_withdraw_item_post(
            &self.configuration,
            name,
            items.to_vec(),
        )
    }

    pub fn deposit_gold(
        &self,
        name: &str,
        quantity: u32,
    ) -> Result<
        BankGoldTransactionResponseSchema,
        Error<ActionDepositBankGoldMyNameActionBankDepositGoldPostError>,
    > {
        let s = DepositWithdrawGoldSchema { quantity };
        action_deposit_bank_gold_my_name_action_bank_deposit_gold_post(&self.configuration, name, s)
    }

    pub fn withdraw_gold(
        &self,
        name: &str,
        quantity: u32,
    ) -> Result<
        BankGoldTransactionResponseSchema,
        Error<ActionWithdrawBankGoldMyNameActionBankWithdrawGoldPostError>,
    > {
        let s = DepositWithdrawGoldSchema { quantity };
        action_withdraw_bank_gold_my_name_action_bank_withdraw_gold_post(
            &self.configuration,
            name,
            s,
        )
    }

    pub fn expand_bank(
        &self,
        name: &str,
    ) -> Result<
        BankExtensionTransactionResponseSchema,
        Error<ActionBuyBankExpansionMyNameActionBankBuyExpansionPostError>,
    > {
        action_buy_bank_expansion_my_name_action_bank_buy_expansion_post(&self.configuration, name)
    }

    pub fn equip(
        &self,
        name: &str,
        item_code: &str,
        slot: ItemSlot,
        quantity: Option<u32>,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        let mut schema = EquipSchema::new(item_code.to_string(), slot);
        schema.quantity = quantity;
        action_equip_item_my_name_action_equip_post(&self.configuration, name, schema)
    }

    pub fn unequip(
        &self,
        name: &str,
        slot: ItemSlot,
        quantity: Option<u32>,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        let mut schema = UnequipSchema::new(slot);
        schema.quantity = quantity;
        action_unequip_item_my_name_action_unequip_post(&self.configuration, name, schema)
    }

    pub fn use_item(
        &self,
        name: &str,
        item_code: &str,
        quantity: u32,
    ) -> Result<UseItemResponseSchema, Error<ActionUseItemMyNameActionUsePostError>> {
        let schema = SimpleItemSchema {
            code: item_code.to_owned(),
            quantity,
        };
        action_use_item_my_name_action_use_post(&self.configuration, name, schema)
    }

    pub fn accept_task(
        &self,
        name: &str,
    ) -> Result<TaskResponseSchema, Error<ActionAcceptNewTaskMyNameActionTaskNewPostError>> {
        action_accept_new_task_my_name_action_task_new_post(&self.configuration, name)
    }

    pub fn cancel_task(
        &self,
        name: &str,
    ) -> Result<TaskCancelledResponseSchema, Error<ActionTaskCancelMyNameActionTaskCancelPostError>>
    {
        action_task_cancel_my_name_action_task_cancel_post(&self.configuration, name)
    }

    pub fn trade_task_item(
        &self,
        name: &str,
        item_code: &str,
        quantity: u32,
    ) -> Result<TaskTradeResponseSchema, Error<ActionTaskTradeMyNameActionTaskTradePostError>> {
        action_task_trade_my_name_action_task_trade_post(
            &self.configuration,
            name,
            SimpleItemSchema::new(item_code.to_owned(), quantity),
        )
    }

    pub fn complete_task(
        &self,
        name: &str,
    ) -> Result<RewardDataResponseSchema, Error<ActionCompleteTaskMyNameActionTaskCompletePostError>>
    {
        action_complete_task_my_name_action_task_complete_post(&self.configuration, name)
    }

    pub fn exchange_tasks_coins(
        &self,
        name: &str,
    ) -> Result<RewardDataResponseSchema, Error<ActionTaskExchangeMyNameActionTaskExchangePostError>>
    {
        action_task_exchange_my_name_action_task_exchange_post(&self.configuration, name)
    }

    pub fn npc_buy(
        &self,
        name: &str,
        code: String,
        quantity: u32,
    ) -> Result<
        NpcMerchantTransactionResponseSchema,
        Error<ActionNpcBuyItemMyNameActionNpcBuyPostError>,
    > {
        let schema = NpcMerchantBuySchema::new(code, quantity);
        action_npc_buy_item_my_name_action_npc_buy_post(&self.configuration, name, schema)
    }

    pub fn npc_sell(
        &self,
        name: &str,
        code: String,
        quantity: u32,
    ) -> Result<
        NpcMerchantTransactionResponseSchema,
        Error<ActionNpcSellItemMyNameActionNpcSellPostError>,
    > {
        let schema = NpcMerchantBuySchema::new(code, quantity);
        action_npc_sell_item_my_name_action_npc_sell_post(&self.configuration, name, schema)
    }

    pub fn give_item(
        &self,
        name: &str,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<GiveItemResponseSchema, Error<ActionGiveItemsMyNameActionGiveItemPostError>> {
        let schema = GiveItemsSchema {
            items: items.to_vec(),
            character: character.to_string(),
        };
        action_give_items_my_name_action_give_item_post(&self.configuration, name, schema)
    }

    pub fn give_gold(
        &self,
        name: &str,
        quantity: u32,
        character: &str,
    ) -> Result<GiveGoldResponseSchema, Error<ActionGiveGoldMyNameActionGiveGoldPostError>> {
        let schema = GiveGoldSchema {
            quantity,
            character: character.to_string(),
        };
        action_give_gold_my_name_action_give_gold_post(&self.configuration, name, schema)
    }

    pub fn ge_buy_order(
        &self,
        name: &str,
        id: &str,
        quantity: u32,
    ) -> Result<
        GeTransactionResponseSchema,
        Error<ActionGeBuyItemMyNameActionGrandexchangeBuyPostError>,
    > {
        let schema = GeBuyOrderSchema::new(id.to_owned(), quantity);
        action_ge_buy_item_my_name_action_grandexchange_buy_post(&self.configuration, name, schema)
    }

    pub fn ge_create_order(
        &self,
        name: &str,
        item_code: &str,
        quantity: u32,
        price: u32,
    ) -> Result<
        GeCreateOrderTransactionResponseSchema,
        Error<ActionGeCreateSellOrderMyNameActionGrandexchangeCreateSellOrderPostError>,
    > {
        let schema = GeOrderCreationrSchema::new(item_code.to_owned(), quantity, price);
        action_ge_create_sell_order_my_name_action_grandexchange_create_sell_order_post(
            &self.configuration,
            name,
            schema,
        )
    }

    pub fn ge_cancel_order(
        &self,
        name: &str,
        id: &str,
    ) -> Result<
        GeTransactionResponseSchema,
        Error<ActionGeCancelOrderMyNameActionGrandexchangeCancelPostError>,
    > {
        action_ge_cancel_order_my_name_action_grandexchange_cancel_post(
            &self.configuration,
            name,
            GeCancelOrderSchema::new(id.to_owned()),
        )
    }

    //pub fn christmas_exchange(
    //    &self,
    //    name: &str,
    //) -> Result<
    //    RewardDataResponseSchema,
    //    Error<ActionChristmasExchangeMyNameActionChristmasExchangePostError>,
    //> {
    //    action_christmas_exchange_my_name_action_christmas_exchange_post(&self.configuration, name)
    //}
}
