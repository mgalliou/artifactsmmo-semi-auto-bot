use artifactsmmo_sdk::Account;

pub mod artifactsmmo_sdk {
    use std::{thread::sleep, time::Duration};

    use artifactsmmo_openapi::{
        apis::{
            characters_api,
            configuration::Configuration,
            my_characters_api::{
                self, get_my_characters_my_characters_get, ActionFightMyNameActionFightPostError,
                ActionGatheringMyNameActionGatheringPostError, GetMyCharactersMyCharactersGetError,
            },
            Error,
        },
        models::{CharacterFightResponseSchema, CharacterSchema, SkillResponseSchema},
    };

    impl Account {
        pub async fn get_character(
            &self,
            index: usize,
        ) -> Result<Character, Error<GetMyCharactersMyCharactersGetError>> {
            let chars = match get_my_characters_my_characters_get(&self.configuration).await {
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
        pub async fn remaining_cooldown(
            &self,
        ) -> Result<i32, Error<characters_api::GetCharacterCharactersNameGetError>> {
            match characters_api::get_character_characters_name_get(
                &self.account.configuration,
                &self.name,
            )
            .await
            {
                Ok(res) => Ok(res.data.cooldown),
                Err(e) => Err(e),
            }
        }
        pub async fn fight(
            &self,
        ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>>
        {
            let res = my_characters_api::action_fight_my_name_action_fight_post(
                &self.account.configuration,
                &self.name,
            )
            .await;
            match res {
                Ok(ref res) => println!("{} fought and {:?}", self.name, res.data.fight.result),
                Err(_) => println!("error during fight"),
            };
            res
        }

        pub async fn gather(
            &self,
        ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>>
        {
            let res = my_characters_api::action_gathering_my_name_action_gathering_post(
                &self.account.configuration,
                &self.name,
            )
            .await;
            match res {
                Ok(ref res) => println!("{} gathered {:?}", self.name, res.data.details.items),
                Err(_) => println!("error during gathering"),
            };
            res
        }

        fn from_schema(value: CharacterSchema, account: Account) -> Self {
            Character {
                account,
                name: value.name,
                level: value.level,
            }
        }

        pub async fn fight_until_unsuccessful(&self) {
            loop {
                let res = self.fight().await;
                match res {
                    Ok(res) => {
                        let s = res.data.cooldown.remaining_seconds.try_into().unwrap();
                        println!("cooling for {} secondes", s);
                        sleep(Duration::from_secs(s));
                    }
                    Err(res) => match res {
                        Error::Reqwest(_) => return,
                        Error::Serde(_) => return,
                        Error::Io(_) => return,
                        Error::ResponseError(res) => match res.entity {
                            Some(e) => match e {
                                ActionFightMyNameActionFightPostError::Status499() => {
                                    let s = Duration::from_secs(
                                        self.remaining_cooldown()
                                            .await
                                            .unwrap()
                                            .try_into()
                                            .unwrap(),
                                    );
                                    sleep(s)
                                }
                                _ => { println!("{:?}", e);
                                    return; 
                                }
                            },
                            None => return,
                        },
                    },
                }
            }
        }

        pub async fn gather_until_unsucessful(&self) {
            loop {
                let res = self.gather().await;
                match res {
                    Ok(res) => {
                        let s = res.data.cooldown.remaining_seconds.try_into().unwrap();
                        println!("cooling for {} secondes", s);
                        sleep(Duration::from_secs(s));
                    }
                    Err(res) => match res {
                        Error::Reqwest(_) => return,
                        Error::Serde(_) => return,
                        Error::Io(_) => return,
                        Error::ResponseError(res) => match res.entity {
                            Some(e) => match e {
                                ActionGatheringMyNameActionGatheringPostError::Status499() => {
                                    let s = Duration::from_secs(
                                        self.remaining_cooldown()
                                            .await
                                            .unwrap()
                                            .try_into()
                                            .unwrap(),
                                    );
                                    sleep(s)
                                }
                                _ => { println!("{:?}", e);
                                    return; 
                                }
                            },
                            None => return,
                        },
                    },
                }
                // if let Ok(ref res) = res {
                //     let s = res.data.cooldown.remaining_seconds.try_into().unwrap();
                //     println!("cooling for {} secondes", s);
                //     sleep(Duration::new(s, 0));
                // }
                // if let Err(ref res) = res {
                //     let tmp = res.source().unwrap()
                //     return;
                // }
            }
        }
    }
}

async fn run() {
    let base_url = "https://api.artifactsmmo.com";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InBvZEppbyIsInBhc3N3b3JkX2NoYW5nZWQiOiIifQ.Qy1Hm2-QYm84O_9aLP076TczjYDCpSuZ75dKkh9toUY";
    let account = Account::new(base_url, token);
    let char1 = account.get_character(1).await;
    let char2 = account.get_character(2).await;
    let handle1 = tokio::spawn(async {
        char1.unwrap().fight_until_unsuccessful().await;
    });
    let handle2 = tokio::spawn(async {
        char2.unwrap().fight_until_unsuccessful().await;
    });                                   
    handle1.await.unwrap();
    handle2.await.unwrap();
}

#[tokio::main]
async fn main() {
    run().await;
}
