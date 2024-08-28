use artifactsmmo_playground::artifactsmmo_sdk::{account::Account, character::Role};
use std::thread;

fn run() {
    let base_url = "https://api.artifactsmmo.com";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InBvZEppbyIsInBhc3N3b3JkX2NoYW5nZWQiOiIifQ.Qy1Hm2-QYm84O_9aLP076TczjYDCpSuZ75dKkh9toUY";
    let account = Account::new(base_url, token);
    let mut char1 = account.get_character(1).unwrap();
    let mut char2 = account.get_character(2).unwrap();
    let mut char3 = account.get_character(3).unwrap();
    let mut char4 = account.get_character(4).unwrap();
    let mut char5 = account.get_character(5).unwrap();

    let t1 = thread::Builder::new()
        .name(char1.name.to_string())
        .spawn(move || {
            char1.run(Role::Fighter);
        })
        .unwrap();
    let t2 = thread::Builder::new()
        .name(char2.name.to_string())
        .spawn(move || {
            char2.run(Role::Miner);
        })
        .unwrap();
    let t3 = thread::Builder::new()
        .name(char3.name.to_string())
        .spawn(move || char3.run(Role::Woodcutter))
        .unwrap();
    let t4 = thread::Builder::new()
        .name(char4.name.to_string())
        .spawn(move || char4.run(Role::Fisher))
        .unwrap();
    let t5 = thread::Builder::new()
        .name(char5.name.to_string())
        .spawn(move || char5.run(Role::Miner))
        .unwrap();
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();
    t5.join().unwrap();
}

fn main() {
    run()
}
