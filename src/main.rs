use artifactsmmo_playground::artifactsmmo_sdk::account::Account;
use std::thread;

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
        char1.fight_until_unsuccessful(1, -1);
    });
    let t2 = thread::spawn(move || {
        char2.gather_until_code("iron_ore");
        //char2.craft_all_repeat("copper")
    });
    let t3 = thread::spawn(move || {
        char3.gather_until_code("spruce_wood");
        //char3.craft_all_repeat("ash_plank")
    });
    let t4 = thread::spawn(move || {
        char4.gather_until_code("golden_shrimp");
    });
    let t5 = thread::spawn(move || {
        //char5.gather_until_unsuccessful(6, 1);
        char5.gather_until_code("ash_wood");
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
