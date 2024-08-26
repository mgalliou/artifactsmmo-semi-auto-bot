use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        my_characters_api::{
            action_crafting_my_name_action_crafting_post,
            action_deposit_bank_my_name_action_bank_deposit_post,
            action_equip_item_my_name_action_equip_post, action_fight_my_name_action_fight_post,
            action_gathering_my_name_action_gathering_post, action_move_my_name_action_move_post,
            action_unequip_item_my_name_action_unequip_post,
            action_withdraw_bank_my_name_action_bank_withdraw_post,
            get_my_characters_my_characters_get, ActionCraftingMyNameActionCraftingPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionEquipItemMyNameActionEquipPostError, ActionFightMyNameActionFightPostError,
            ActionGatheringMyNameActionGatheringPostError, ActionMoveMyNameActionMovePostError,
            ActionUnequipItemMyNameActionUnequipPostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
            GetMyCharactersMyCharactersGetError,
        },
        Error,
    },
    models::{
        equip_schema, unequip_schema, BankItemTransactionResponseSchema,
        CharacterFightResponseSchema, CharacterMovementResponseSchema, CraftingSchema,
        DestinationSchema, EquipSchema, EquipmentResponseSchema, MyCharactersListSchema,
        SimpleItemSchema, SkillResponseSchema, UnequipSchema,
    },
};

#[derive(Clone)]
pub struct MyCharacterApi {
    pub configuration: Configuration,
}

impl MyCharacterApi {
    pub fn new(base_path: &str, token: &str) -> MyCharacterApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        MyCharacterApi { configuration }
    }

    pub fn characters(
        &self,
    ) -> Result<MyCharactersListSchema, Error<GetMyCharactersMyCharactersGetError>> {
        get_my_characters_my_characters_get(&self.configuration)
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

    pub fn equip(
        &self,
        name: &str,
        code: &str,
        slot: equip_schema::Slot,
        quantity: Option<i32>,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        let mut schema = EquipSchema::new(code.to_string(), slot);
        schema.quantity = quantity;
        action_equip_item_my_name_action_equip_post(&self.configuration, name, schema)
    }

    pub fn unequip(
        &self,
        name: &str,
        slot: unequip_schema::Slot,
        quantity: Option<i32>,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        let mut schema = UnequipSchema::new(slot);
        schema.quantity = quantity;
        action_unequip_item_my_name_action_unequip_post(&self.configuration, name, schema)
    }

    pub fn deposit(
        &self,
        name: &str,
        item_code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        action_deposit_bank_my_name_action_bank_deposit_post(
            &self.configuration,
            name,
            SimpleItemSchema::new(item_code.to_owned(), quantity),
        )
    }

    pub fn withdraw(
        &self,
        name: &str,
        item_code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        action_withdraw_bank_my_name_action_bank_withdraw_post(
            &self.configuration,
            name,
            SimpleItemSchema::new(item_code.to_owned(), quantity),
        )
    }

    pub fn all(
        &self,
    ) -> Result<MyCharactersListSchema, Error<GetMyCharactersMyCharactersGetError>> {
        get_my_characters_my_characters_get(&self.configuration)
    }
}
