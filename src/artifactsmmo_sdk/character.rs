use std::{option::Option, thread::sleep, time::Duration, vec::Vec};

use super::{
    account::Account,
    api::{characters::CharactersApi, items::ItemsApi, my_character::MyCharacterApi},
};
use artifactsmmo_openapi::{
    apis::{
        characters_api::GetCharacterCharactersNameGetError,
        my_characters_api::{
            ActionCraftingMyNameActionCraftingPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionFightMyNameActionFightPostError, ActionGatheringMyNameActionGatheringPostError,
            ActionMoveMyNameActionMovePostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        craft_schema::Skill::{
            Cooking, Gearcrafting, Jewelrycrafting, Mining, Weaponcrafting, Woodcutting,
        },
        ActionItemBankResponseSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, InventorySlot, SimpleItemSchema, SkillResponseSchema,
    },
};
use reqwest::StatusCode;

pub struct Character {
    api: CharactersApi,
    my_api: MyCharacterApi,
    items_api: ItemsApi,
    name: String,
}

impl Character {
    pub fn new(account: &Account, name: &str) -> Character {
        Character {
            api: CharactersApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            my_api: MyCharacterApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            items_api: ItemsApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            name: name.to_owned(),
        }
    }

    pub fn move_to(
        &self,
        x: i32,
        y: i32,
    ) -> Result<CharacterMovementResponseSchema, Error<ActionMoveMyNameActionMovePostError>> {
        let res = self.my_api.move_to(&self.name, x, y);
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
        let res = self.my_api.fight(&self.name);
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
        let res = self.my_api.gather(&self.name);
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
        let res = self.my_api.craft(&self.name, code, quantity);
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
        let info = self.items_api.info(code).unwrap();
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
        code: &str,
        quantity: i32,
    ) -> Result<
        ActionItemBankResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        let res = self.my_api.deposit(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: deposited {} {}", self.name, code, quantity);
                self.cool_down(res.data.cooldown.remaining_seconds)
            }
            Err(ref e) => println!("{}: error while depositing: {}", self.name, e),
        }
        res
    }

    pub fn withdraw(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<
        ActionItemBankResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        let res = self.my_api.withdraw(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: withdrawed {} {}", self.name, code, quantity);
                self.cool_down(res.data.cooldown.remaining_seconds)
            }
            Err(ref e) => println!("{}: error while withdrawing: {}", self.name, e),
        }
        res
    }

    pub fn deposit_all(&self) {
        for i in self.inventory() {
            if i.quantity > 1 {
                let _ = self.deposit(&i.code, i.quantity);
            }
        }
    }

    fn cool_down(&self, s: i32) {
        println!("{}: cooling down for {} secondes", self.name, s);
        sleep(Duration::from_secs(s.try_into().unwrap()));
    }

    pub fn inventory(&self) -> Vec<InventorySlot> {
        let char = self.api.get(&self.name).unwrap();
        char.data.inventory.unwrap()
    }

    pub fn inventory_max_items(&self) -> i32 {
        let char = self.api.get(&self.name).unwrap();
        char.data.inventory_max_items
    }

    pub fn remaining_cooldown(&self) -> Result<i32, Error<GetCharacterCharactersNameGetError>> {
        match self.api.get(&self.name) {
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

    pub fn craft_all_repeat(&self, code: &str) {
        loop {
            self.move_to_bank();
            self.deposit_all();
            let required_items = self.get_required_item_for(code).unwrap();
            let info = self.items_api.info(code).unwrap();
            for i in required_items {
                let _ = self.withdraw(&i.code, self.inventory_max_items());
            }
            let _ = match info.data.item.craft.unwrap().unwrap().skill.unwrap() {
                Weaponcrafting => self.move_to(2, 1),
                Gearcrafting => self.move_to(2, 2),
                Jewelrycrafting => self.move_to(1, 3),
                Cooking => self.move_to(1, 1),
                Woodcutting => self.move_to(-2, -3),
                Mining => self.move_to(1, 5),
            };
            let _ = self.craft_all(code);
        }
    }

    fn get_required_item_for(&self, code: &str) -> Option<Vec<SimpleItemSchema>> {
        match self.items_api.info(code) {
            Ok(info) => match info.data.item.craft {
                Some(Some(craft)) => craft.items,
                Some(None) => None,
                None => None,
            },
            Err(_) => None,
        }
    }
}
