use std::thread;

use artifactsmmo_sdk::Account;

pub mod artifactsmmo_sdk {
    use std::{thread::sleep, time::Duration};

    use artifactsmmo_openapi::{
        apis::{
            characters_api,
            configuration::Configuration,
            my_characters_api::{
                self, get_my_characters_my_characters_get, ActionFightMyNameActionFightPostError,
                ActionGatheringMyNameActionGatheringPostError, ActionMoveMyNameActionMovePostError,
                GetMyCharactersMyCharactersGetError,
            },
            Error,
        },
        models::{
            CharacterFightResponseSchema, CharacterMovementResponseSchema, CharacterSchema,
            DestinationSchema, SkillResponseSchema,
        },
    };

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
        pub fn remaining_cooldown(
            &self,
        ) -> Result<i32, Error<characters_api::GetCharacterCharactersNameGetError>> {
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
                Ok(ref res) => println!("{} fought and {:?}", self.name, res.data.fight.result),
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
                Ok(ref res) => println!("{}: gathered {:?}", self.name, res.data.details.items),
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
                Ok(_) => println!("{}: moved to {},{}", self.name, x, y),
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

        pub fn fight_until_unsuccessful(&self) {
            loop {
                match self.fight() {
                    Ok(res) => {
                        self.cool_down(res.data.cooldown.remaining_seconds);
                    }
                    Err(res) => match res {
                        Error::ResponseError(res) => match res.entity {
                            Some(e) => match e {
                                ActionFightMyNameActionFightPostError::UnknownValue(json) => {
                                    self.handle_error(json);
                                }
                                _ => {
                                    println!("unrecoverable error: {:?}", e);
                                    return;
                                }
                            },
                            None => return,
                        },
                        _ => return,
                    },
                }
            }
        }

        pub fn gather_until_unsuccessful(&self) {
            loop {
                match self.gather() {
                    Ok(res) => {
                        self.cool_down(res.data.cooldown.remaining_seconds);
                    }
                    Err(res) => match res {
                        Error::ResponseError(res) => match res.entity {
                            Some(e) => match e {
                                ActionGatheringMyNameActionGatheringPostError::UnknownValue(
                                    json,
                                ) => {
                                    self.handle_error(json);
                                }
                                _ => {
                                    println!("unrecoverable error: {:?}", e);
                                    return;
                                }
                            },
                            None => return,
                        },
                        _ => return,
                    },
                }
            }
        }

        fn handle_error(&self, json: serde_json::Value) {
            let code = json.get("error").unwrap().get("code").unwrap();
            println!("code: {}", code);
            if code == 499 {
                println!("{}: needs to cooldown", self.name);
                self.cool_down(self.remaining_cooldown().unwrap());
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
        char1.fight_until_unsuccessful();
    });
    let t2 = thread::spawn(move || {
        char2.gather_until_unsuccessful();
    });
    let t3 = thread::spawn(move || {
        char3.gather_until_unsuccessful();
    });
    let t4 = thread::spawn(move || {
        char4.gather_until_unsuccessful();
    });
    let t5 = thread::spawn(move || {
        char5.gather_until_unsuccessful();
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
