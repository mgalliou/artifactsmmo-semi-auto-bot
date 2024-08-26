use super::{
    account::Account,
    api::{
        characters::CharactersApi, items::ItemsApi, monsters::MonstersApi,
        my_character::MyCharacterApi, resources::ResourcesApi,
    },
    maps::Maps,
};
use artifactsmmo_openapi::{
    apis::{
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
        BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, InventorySlot, MapSchema, SimpleItemSchema,
        SkillResponseSchema,
    },
};
use chrono::{DateTime, FixedOffset};
use reqwest::StatusCode;
use std::{cmp::Ordering, option::Option, thread::sleep, time::Duration, vec::Vec};

pub struct Character {
    account: Account,
    api: CharactersApi,
    my_api: MyCharacterApi,
    maps: Maps,
    items_api: ItemsApi,
    resources_api: ResourcesApi,
    monsters_api: MonstersApi,
    name: String,
}

impl Character {
    pub fn new(account: &Account, name: &str) -> Character {
        Character {
            account: account.clone(),
            api: CharactersApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            my_api: MyCharacterApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            maps: Maps::new(account),
            items_api: ItemsApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            resources_api: ResourcesApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            monsters_api: MonstersApi::new(
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
                self.cool_down(Duration::from_secs(
                    res.data.cooldown.remaining_seconds.try_into().unwrap(),
                ))
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
                self.cool_down(Duration::from_secs(
                    res.data.cooldown.remaining_seconds.try_into().unwrap(),
                ))
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
                self.cool_down(Duration::from_secs(
                    res.data.cooldown.remaining_seconds.try_into().unwrap(),
                ))
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
                self.cool_down(Duration::from_secs(
                    res.data.cooldown.remaining_seconds.try_into().unwrap(),
                ))
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
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        let res = self.my_api.deposit(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: deposited {} {}", self.name, code, quantity);
                self.cool_down(Duration::from_secs(
                    res.data.cooldown.remaining_seconds.try_into().unwrap(),
                ))
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
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        let res = self.my_api.withdraw(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: withdrawed {} {}", self.name, code, quantity);
                self.cool_down(Duration::from_secs(
                    res.data.cooldown.remaining_seconds.try_into().unwrap(),
                ))
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

    fn cool_down(&self, s: Duration) {
        println!(
            "{}: cooling down for {}.{} secondes",
            self.name,
            s.as_secs(),
            s.subsec_millis()
        );
        sleep(s);
    }

    pub fn inventory(&self) -> Vec<InventorySlot> {
        let char = self.api.get(&self.name).unwrap();
        char.data.inventory.unwrap()
    }

    pub fn inventory_max_items(&self) -> i32 {
        let char = self.api.get(&self.name).unwrap();
        char.data.inventory_max_items
    }

    pub fn cooldown_expiration(&self) -> Option<DateTime<FixedOffset>> {
        match self.api.get(&self.name) {
            Ok(res) => match res.data.cooldown_expiration {
                Some(cd) => match DateTime::parse_from_rfc3339(&cd) {
                    Ok(cd) => Some(cd),
                    Err(_) => None,
                },
                None => None,
            },
            Err(_) => None,
        }
    }

    pub fn remaining_cooldown(&self) -> Duration {
        if let Some(server_time) = self.account.server_time() {
            if let Some(cd) = self.cooldown_expiration() {
                if server_time.cmp(&cd) == Ordering::Less {
                    return (cd - server_time).to_std().unwrap();
                }
            }
        };
        Duration::default()
    }

    fn ressources_dropping(&self, code: &str) -> Option<Vec<String>> {
        let mut codes: Vec<String> = vec![];

        if let Ok(resources) = self
            .resources_api
            .all(None, None, None, Some(code), None, None)
        {
            for r in resources.data {
                codes.push(r.code)
            }
            return Some(codes);
        }
        None
    }

    fn closest(&self, maps: Vec<MapSchema>) -> Option<MapSchema> {
        let (x, y) = self.coordinates();
        self.maps.closest_from_amoung(x, y, maps)
    }

    fn get_cordinate_for_drop(&self, code: &str) -> Option<(i32, i32)> {
        let (mut x, mut y): (i32, i32) = (0, 0);

        if let Some(resources) = self.ressources_dropping(code) {
            for r in resources {
                (x, y) = self.get_cordinate_for_resources(&r).unwrap();
            }
            return Some((x, y));
        }
        None
    }

    fn get_cordinate_for_resources(&self, code: &str) -> Option<(i32, i32)> {
        if let Ok(maps) = self.maps.get_cordinate_for_resources(code) {
            let map = self.closest(maps.data).unwrap();
            return Some((map.x, map.y));
        }
        None
    }

    pub fn fight_until_unsuccessful(&self, x: i32, y: i32) {
        let _ = self.move_to(x, y);
        loop {
            if let Err(Error::ResponseError(res)) = self.fight() {
                if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                    println!("{}: needs to cooldown", self.name);
                    self.cool_down(self.remaining_cooldown());
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

    pub fn gather_until_at(&self, x: i32, y: i32) {
        let _ = self.move_to(x, y);
        loop {
            if let Err(Error::ResponseError(res)) = self.gather() {
                if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                    println!("{}: needs to cooldown", self.name);
                    self.cool_down(self.remaining_cooldown());
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

    pub fn gather_until_code(&self, code: &str) {
        let (x, y) = self.get_cordinate_for_drop(code).unwrap();
        let _ = self.move_to(x, y);
        loop {
            if let Err(Error::ResponseError(res)) = self.gather() {
                if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                    println!("{}: needs to cooldown", self.name);
                    self.cool_down(self.remaining_cooldown());
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
        self.cool_down(self.remaining_cooldown());
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

    fn coordinates(&self) -> (i32, i32) {
        let data = self.api.get(&self.name).unwrap().data;
        (data.x, data.y)
    }
}
