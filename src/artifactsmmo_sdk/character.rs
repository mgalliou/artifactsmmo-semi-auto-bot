use std::{thread::sleep, time::Duration};

use super::account::Account;
use artifactsmmo_openapi::{
    apis::{
        characters_api::{self, GetCharacterCharactersNameGetError},
        my_characters_api::{
            self as api, ActionCraftingMyNameActionCraftingPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionFightMyNameActionFightPostError, ActionGatheringMyNameActionGatheringPostError,
            ActionMoveMyNameActionMovePostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        ActionItemBankResponseSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, CharacterSchema, CraftingSchema, DestinationSchema,
        InventorySlot, SimpleItemSchema, SkillResponseSchema,
    },
};
use reqwest::StatusCode;

pub struct Character {
    account: Account,
    name: String,
    level: i32,
}

impl Character {
    pub fn from_schema(value: CharacterSchema, account: Account) -> Self {
        Character {
            account,
            name: value.name,
            level: value.level,
        }
    }

    pub fn move_to(
        &self,
        x: i32,
        y: i32,
    ) -> Result<CharacterMovementResponseSchema, Error<ActionMoveMyNameActionMovePostError>> {
        let dest = DestinationSchema::new(x, y);
        let res = api::action_move_my_name_action_move_post(
            &self.account.configuration,
            &self.name,
            dest,
        );
        match res {
            Ok(ref res) => {
                println!("{}: moved to {},{}", self.name, x, y);
                self.cool_down(res.data.cooldown.remaining_seconds)
            }
            Err(ref e) => println!("{}: error while moving: {}", self.name, e),
        }
        res
    }

    fn move_to_bank(&self) {
        let _ = self.move_to(4, 1);
    }

    pub fn fight(
        &self,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        let res =
            api::action_fight_my_name_action_fight_post(&self.account.configuration, &self.name);
        match res {
            Ok(ref res) => {
                println!("{} fought and {:?}", self.name, res.data.fight.result);
                self.cool_down(res.data.cooldown.remaining_seconds);
            }
            Err(ref e) => println!("{}: error during fight: {}", self.name, e),
        };
        res
    }

    pub fn gather(
        &self,
    ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>> {
        let res = api::action_gathering_my_name_action_gathering_post(
            &self.account.configuration,
            &self.name,
        );
        match res {
            Ok(ref res) => {
                println!("{}: gathered {:?}", self.name, res.data.details.items);
                self.cool_down(res.data.cooldown.remaining_seconds);
            }
            Err(ref e) => println!("{}: error during gathering: {}", self.name, e),
        };
        res
    }

    pub fn craft(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        let schema = CraftingSchema {
            code: code.to_owned(),
            quantity: Some(quantity),
        };
        let res = api::action_crafting_my_name_action_crafting_post(
            &self.account.configuration,
            &self.name,
            schema,
        );

        match res {
            Ok(ref res) => {
                println!("{}: crafted {}, {}", self.name, quantity, code);
                self.cool_down(res.data.cooldown.remaining_seconds);
            }
            Err(ref e) => println!("{}: error during crafting: {}", self.name, e),
        };
        res
    }

    pub fn number_in_inventory(&self, code: &str) -> i32 {
        let inv = self.inventory();
        let mut quantity: i32;

        quantity = 0;
        for i in inv {
            if i.code == code {
                quantity += i.quantity;
            }
        }
        quantity
    }

    pub fn craft_all(
        &self,
        code: &str,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        let info = self.account.get_item_info(code).unwrap();
        let mut n = 0;
        let mut new_n;

        for i in info.data.item.craft.unwrap().unwrap().items.unwrap() {
            if i.quantity <= self.number_in_inventory(&i.code) {
                new_n = self.number_in_inventory(&i.code) / i.quantity;
                if n == 0 || new_n < n {
                    n = new_n;
                }
            }
        }
        self.craft(code, n)
    }

    pub fn deposit(
        &self,
        item_code: String,
        quantity: i32,
    ) -> Result<
        ActionItemBankResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        let res = api::action_deposit_bank_my_name_action_bank_deposit_post(
            &self.account.configuration,
            &self.name,
            SimpleItemSchema::new(item_code.clone(), quantity),
        );
        match res {
            Ok(ref res) => {
                println!("{}: deposited {} {}", self.name, item_code, quantity);
                self.cool_down(res.data.cooldown.remaining_seconds)
            }
            Err(ref e) => println!("{}: error while depositing: {}", self.name, e),
        }
        res
    }

    pub fn withdraw(
        &self,
        item_code: String,
        quantity: i32,
    ) -> Result<
        ActionItemBankResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        let res = api::action_withdraw_bank_my_name_action_bank_withdraw_post(
            &self.account.configuration,
            &self.name,
            SimpleItemSchema::new(item_code.clone(), quantity),
        );
        match res {
            Ok(ref res) => {
                println!("{}: withdrawed {} {}", self.name, item_code, quantity);
                self.cool_down(res.data.cooldown.remaining_seconds)
            }
            Err(ref e) => println!("{}: error while withdrawing: {}", self.name, e),
        }
        res
    }

    pub fn deposit_all(&self) {
        for i in self.inventory() {
            if i.quantity > 1 {
                let _ = self.deposit(i.code, i.quantity);
            }
        }
    }

    fn cool_down(&self, s: i32) {
        println!("{}: cooling down for {} secondes", self.name, s);
        sleep(Duration::from_secs(s.try_into().unwrap()));
    }

    pub fn inventory(&self) -> Vec<InventorySlot> {
        let chars = self.account.get_character_by_name(&self.name).unwrap();
        chars.inventory.unwrap()
    }

    pub fn remaining_cooldown(&self) -> Result<i32, Error<GetCharacterCharactersNameGetError>> {
        match characters_api::get_character_characters_name_get(
            &self.account.configuration,
            &self.name,
        ) {
            Ok(res) => Ok(res.data.cooldown),
            Err(e) => Err(e),
        }
    }

    pub fn fight_until_unsuccessful(&self, x: i32, y: i32) {
        let _ = self.move_to(x, y);
        loop {
            if let Err(Error::ResponseError(res)) = self.fight() {
                if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                    println!("{}: needs to cooldown", self.name);
                    self.cool_down(self.remaining_cooldown().unwrap());
                }
                if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
                    println!("{}: inventory is full", self.name);
                    self.move_to_bank();
                    self.deposit_all();
                    let _ = self.move_to(x, y);
                }
            }
        }
    }

    pub fn gather_until_unsuccessful(&self, x: i32, y: i32) {
        let _ = self.move_to(x, y);
        loop {
            if let Err(Error::ResponseError(res)) = self.gather() {
                if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                    println!("{}: needs to cooldown", self.name);
                    self.cool_down(self.remaining_cooldown().unwrap());
                }
                if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
                    println!("{}: inventory is full", self.name);
                    self.move_to_bank();
                    self.deposit_all();
                    let _ = self.move_to(x, y);
                }
            }
        }
    }
}
