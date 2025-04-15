use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        my_characters_api::{
            action_accept_new_task_my_name_action_task_new_post,
            action_buy_bank_expansion_my_name_action_bank_buy_expansion_post,
            action_complete_task_my_name_action_task_complete_post,
            action_crafting_my_name_action_crafting_post,
            action_delete_item_my_name_action_delete_post,
            action_deposit_bank_gold_my_name_action_bank_deposit_gold_post,
            action_deposit_bank_my_name_action_bank_deposit_post,
            action_equip_item_my_name_action_equip_post, action_fight_my_name_action_fight_post,
            action_gathering_my_name_action_gathering_post, action_move_my_name_action_move_post,
            action_recycling_my_name_action_recycling_post, action_rest_my_name_action_rest_post,
            action_task_cancel_my_name_action_task_cancel_post,
            action_task_exchange_my_name_action_task_exchange_post,
            action_task_trade_my_name_action_task_trade_post,
            action_unequip_item_my_name_action_unequip_post,
            action_use_item_my_name_action_use_post,
            action_withdraw_bank_gold_my_name_action_bank_withdraw_gold_post,
            action_withdraw_bank_my_name_action_bank_withdraw_post,
            ActionAcceptNewTaskMyNameActionTaskNewPostError,
            ActionBuyBankExpansionMyNameActionBankBuyExpansionPostError,
            ActionCompleteTaskMyNameActionTaskCompletePostError,
            ActionCraftingMyNameActionCraftingPostError,
            ActionDeleteItemMyNameActionDeletePostError,
            ActionDepositBankGoldMyNameActionBankDepositGoldPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionEquipItemMyNameActionEquipPostError, ActionFightMyNameActionFightPostError,
            ActionGatheringMyNameActionGatheringPostError, ActionMoveMyNameActionMovePostError,
            ActionRecyclingMyNameActionRecyclingPostError, ActionRestMyNameActionRestPostError,
            ActionTaskCancelMyNameActionTaskCancelPostError,
            ActionTaskExchangeMyNameActionTaskExchangePostError,
            ActionTaskTradeMyNameActionTaskTradePostError,
            ActionUnequipItemMyNameActionUnequipPostError, ActionUseItemMyNameActionUsePostError,
            ActionWithdrawBankGoldMyNameActionBankWithdrawGoldPostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
        BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, CharacterRestResponseSchema, CraftingSchema,
        DeleteItemResponseSchema, DepositWithdrawGoldSchema, DestinationSchema, EquipSchema,
        EquipmentResponseSchema, ItemSlot, RecyclingResponseSchema, RecyclingSchema,
        RewardDataResponseSchema, SimpleItemSchema, SkillResponseSchema,
        TaskCancelledResponseSchema, TaskResponseSchema, TaskTradeResponseSchema, UnequipSchema,
        UseItemResponseSchema,
    },
};
use std::sync::Arc;

pub struct MyCharacterApi {
    configuration: Arc<Configuration>,
}

impl MyCharacterApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        MyCharacterApi { configuration }
    }

    pub fn move_to(
        &self,
        name: &str,
        x: i32,
        y: i32,
    ) -> Result<CharacterMovementResponseSchema, Error<ActionMoveMyNameActionMovePostError>> {
        let dest = DestinationSchema::new(x, y);
        action_move_my_name_action_move_post(&self.configuration, name, dest)
    }

    pub fn fight(
        &self,
        name: &str,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        action_fight_my_name_action_fight_post(&self.configuration, name)
    }

    pub fn rest(
        &self,
        name: &str,
    ) -> Result<CharacterRestResponseSchema, Error<ActionRestMyNameActionRestPostError>> {
        action_rest_my_name_action_rest_post(&self.configuration, name)
    }

    pub fn use_item(
        &self,
        name: &str,
        item: &str,
        quantity: i32,
    ) -> Result<UseItemResponseSchema, Error<ActionUseItemMyNameActionUsePostError>> {
        let schema = SimpleItemSchema {
            code: item.to_owned(),
            quantity,
        };
        action_use_item_my_name_action_use_post(&self.configuration, name, schema)
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
        code: &str,
        quantity: i32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        let schema = CraftingSchema {
            code: code.to_owned(),
            quantity: Some(quantity),
        };
        action_crafting_my_name_action_crafting_post(&self.configuration, name, schema)
    }

    pub fn delete(
        &self,
        name: &str,
        code: &str,
        quantity: i32,
    ) -> Result<DeleteItemResponseSchema, Error<ActionDeleteItemMyNameActionDeletePostError>> {
        let schema = SimpleItemSchema {
            code: code.to_owned(),
            quantity,
        };
        action_delete_item_my_name_action_delete_post(&self.configuration, name, schema)
    }

    pub fn recycle(
        &self,
        name: &str,
        code: &str,
        quantity: i32,
    ) -> Result<RecyclingResponseSchema, Error<ActionRecyclingMyNameActionRecyclingPostError>> {
        let schema = RecyclingSchema {
            code: code.to_owned(),
            quantity: Some(quantity),
        };
        action_recycling_my_name_action_recycling_post(&self.configuration, name, schema)
    }

    pub fn equip(
        &self,
        name: &str,
        code: &str,
        slot: ItemSlot,
        quantity: Option<i32>,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        let mut schema = EquipSchema::new(code.to_string(), slot);
        schema.quantity = quantity;
        action_equip_item_my_name_action_equip_post(&self.configuration, name, schema)
    }

    pub fn unequip(
        &self,
        name: &str,
        slot: ItemSlot,
        quantity: Option<i32>,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        let mut schema = UnequipSchema::new(slot);
        schema.quantity = quantity;
        action_unequip_item_my_name_action_unequip_post(&self.configuration, name, schema)
    }

    pub fn deposit(
        &self,
        name: &str,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        action_deposit_bank_my_name_action_bank_deposit_post(
            &self.configuration,
            name,
            SimpleItemSchema::new(code.to_owned(), quantity),
        )
    }

    pub fn withdraw(
        &self,
        name: &str,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        action_withdraw_bank_my_name_action_bank_withdraw_post(
            &self.configuration,
            name,
            SimpleItemSchema::new(code.to_owned(), quantity),
        )
    }

    pub fn deposit_gold(
        &self,
        name: &str,
        quantity: i32,
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
        quantity: i32,
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

    pub fn accept_task(
        &self,
        name: &str,
    ) -> Result<TaskResponseSchema, Error<ActionAcceptNewTaskMyNameActionTaskNewPostError>> {
        action_accept_new_task_my_name_action_task_new_post(&self.configuration, name)
    }

    pub fn complete_task(
        &self,
        name: &str,
    ) -> Result<RewardDataResponseSchema, Error<ActionCompleteTaskMyNameActionTaskCompletePostError>>
    {
        action_complete_task_my_name_action_task_complete_post(&self.configuration, name)
    }

    pub fn cancel_task(
        &self,
        name: &str,
    ) -> Result<TaskCancelledResponseSchema, Error<ActionTaskCancelMyNameActionTaskCancelPostError>>
    {
        action_task_cancel_my_name_action_task_cancel_post(&self.configuration, name)
    }

    pub fn trade_task(
        &self,
        name: &str,
        code: &str,
        quantity: i32,
    ) -> Result<TaskTradeResponseSchema, Error<ActionTaskTradeMyNameActionTaskTradePostError>> {
        action_task_trade_my_name_action_task_trade_post(
            &self.configuration,
            name,
            SimpleItemSchema::new(code.to_owned(), quantity),
        )
    }

    pub fn task_exchange(
        &self,
        name: &str,
    ) -> Result<RewardDataResponseSchema, Error<ActionTaskExchangeMyNameActionTaskExchangePostError>>
    {
        action_task_exchange_my_name_action_task_exchange_post(&self.configuration, name)
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
