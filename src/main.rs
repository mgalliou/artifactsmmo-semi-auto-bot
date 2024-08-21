use std::thread;

use artifactsmmo_sdk::Account;

pub mod artifactsmmo_sdk {
    use std::{thread::sleep, time::Duration};

    use artifactsmmo_openapi::{
        apis::{
            characters_api::{self, GetCharacterCharactersNameGetError},
            configuration::Configuration,
            my_characters_api::{
                self, get_my_characters_my_characters_get,
                ActionDepositBankMyNameActionBankDepositPostError,
                ActionFightMyNameActionFightPostError,
                ActionGatheringMyNameActionGatheringPostError, ActionMoveMyNameActionMovePostError,
                GetMyCharactersMyCharactersGetError,
            },
            Error,
        },
        models::{
            ActionItemBankResponseSchema, CharacterFightResponseSchema,
            CharacterMovementResponseSchema, CharacterSchema, DestinationSchema, InventorySlot,
            SimpleItemSchema, SkillResponseSchema,
        },
    };
    use reqwest::StatusCode;

    pub trait UnkownError {
        fn get_json(&self) -> Option<&serde_json::Value>;
    }

    impl UnkownError for ActionFightMyNameActionFightPostError {
        fn get_json(&self) -> Option<&serde_json::Value> {
            match self {
                ActionFightMyNameActionFightPostError::UnknownValue(s) => Some(s),
                _ => None,
            }
        }
    }
    impl UnkownError for ActionGatheringMyNameActionGatheringPostError {
        fn get_json(&self) -> Option<&serde_json::Value> {
            match self {
                ActionGatheringMyNameActionGatheringPostError::UnknownValue(s) => Some(s),
                _ => None,
            }
        }
    }

    impl Account {
        pub fn get_character(
            &self,
            index: usize,
        ) -> Result<Character, Error<GetMyCharactersMyCharactersGetError>> {
            let chars = match get_my_characters_my_characters_get(&self.configuration) {
                Ok(c) => Ok(c.data),
                Err(e) => Err(e),
            };
            match chars {
                Ok(c) => Ok(Character::from_schema(c[index - 1].clone(), self.clone())),
                Err(e) => Err(e),
            }
        }

        pub fn get_character_by_name(&self, name: &str) -> Option<CharacterSchema> {
            let chars = get_my_characters_my_characters_get(&self.configuration).unwrap();
            chars.data.iter().find(|c| c.name == name).cloned()
        }
    }

    #[derive(Clone)]
    pub struct Account {
        pub configuration: Configuration,
    }

    impl Account {
        pub fn new(base_url: &str, token: &str) -> Account {
            let mut configuration = Configuration::new();
            configuration.base_path = base_url.to_owned();
            configuration.bearer_access_token = Some(token.to_owned());
            Account { configuration }
        }
    }

    pub struct Character {
        account: Account,
        name: String,
        level: i32,
    }

    impl Character {
        pub fn remaining_cooldown(&self) -> Result<i32, Error<GetCharacterCharactersNameGetError>> {
            match characters_api::get_character_characters_name_get(
                &self.account.configuration,
                &self.name,
            ) {
                Ok(res) => Ok(res.data.cooldown),
                Err(e) => Err(e),
            }
        }

        pub fn fight(
            &self,
        ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>>
        {
            let res = my_characters_api::action_fight_my_name_action_fight_post(
                &self.account.configuration,
                &self.name,
            );
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
        ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>>
        {
            let res = my_characters_api::action_gathering_my_name_action_gathering_post(
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

        pub fn move_to(
            &self,
            x: i32,
            y: i32,
        ) -> Result<CharacterMovementResponseSchema, Error<ActionMoveMyNameActionMovePostError>>
        {
            let dest = DestinationSchema::new(x, y);
            let res = my_characters_api::action_move_my_name_action_move_post(
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

        fn from_schema(value: CharacterSchema, account: Account) -> Self {
            Character {
                account,
                name: value.name,
                level: value.level,
            }
        }

        pub fn fight_until_unsuccessful(&self, x: i32, y: i32) {
            loop {
                if let Err(Error::ResponseError(res)) = self.fight() {
                    if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                        println!("{}: needs to cooldown", self.name);
                        self.cool_down(self.remaining_cooldown().unwrap());
                    }
                    if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
                        println!("{}: inventory is full", self.name);
                        let _ = self.move_to(4, 1);
                        self.deposit_all();
                        let _ = self.move_to(x, y);
                    }
                }
            }
        }

        pub fn gather_until_unsuccessful(&self, x: i32, y: i32) {
            loop {
                if let Err(Error::ResponseError(res)) = self.gather() {
                    if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
                        println!("{}: needs to cooldown", self.name);
                        self.cool_down(self.remaining_cooldown().unwrap());
                    }
                    if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
                        println!("{}: inventory is full", self.name);
                        let _ = self.move_to(4, 1);
                        self.deposit_all();
                        let _ = self.move_to(x, y);
                    }
                }
            }
        }

        pub fn deposit(
            &self,
            item_code: String,
            quantity: i32,
        ) -> Result<
            ActionItemBankResponseSchema,
            Error<ActionDepositBankMyNameActionBankDepositPostError>,
        > {
            let res = my_characters_api::action_deposit_bank_my_name_action_bank_deposit_post(
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

        pub fn inventory(&self) -> Vec<InventorySlot> {
            let chars = self.account.get_character_by_name(&self.name).unwrap();
            chars.inventory.unwrap()
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
    }
}

fn run() {
    let base_url = "https://api.artifactsmmo.com";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InBvZEppbyIsInBhc3N3b3JkX2NoYW5nZWQiOiIifQ.Qy1Hm2-QYm84O_9aLP076TczjYDCpSuZ75dKkh9toUY";
    let account = Account::new(base_url, token);
    let char1 = account.get_character(1).unwrap();
    let char2 = account.get_character(2).unwrap();
    let char3 = account.get_character(3).unwrap();
    let char4 = account.get_character(4).unwrap();
    let char5 = account.get_character(5).unwrap();

    let t1 = thread::spawn(move || {
        char1.fight_until_unsuccessful(0, -1);
    });
    let t2 = thread::spawn(move || {
        char2.gather_until_unsuccessful(2, 0);
    });
    let t3 = thread::spawn(move || {
        char3.gather_until_unsuccessful(6, 1);
    });
    let t4 = thread::spawn(move || {
        char4.gather_until_unsuccessful(4, 2);
    });
    let t5 = thread::spawn(move || {
        char5.gather_until_unsuccessful(2, 0);
    });
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();
    t5.join().unwrap();
}

fn main() {
    run()
}
