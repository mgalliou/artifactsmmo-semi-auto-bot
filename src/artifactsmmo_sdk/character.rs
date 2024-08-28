use super::{
    account::Account,
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    bank::Bank,
    items::{Items, Type},
    maps::Maps,
    monsters::Monsters,
    resources::Resources,
    skill::Skill,
};
use artifactsmmo_openapi::{
    apis::{
        characters_api::GetCharacterCharactersNameGetError,
        my_characters_api::{
            ActionCraftingMyNameActionCraftingPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionEquipItemMyNameActionEquipPostError, ActionFightMyNameActionFightPostError,
            ActionGatheringMyNameActionGatheringPostError,
            ActionUnequipItemMyNameActionUnequipPostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        equip_schema::{self, Slot},
        unequip_schema, BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterResponseSchema, EquipmentResponseSchema, InventorySlot, ItemSchema, MapSchema,
        SingleItemSchema, SkillResponseSchema,
    },
};
use chrono::{DateTime, Utc};
use log::{info, warn};
use std::{cmp::Ordering, option::Option, thread::sleep, time::Duration, vec::Vec};

pub struct Character {
    account: Account,
    api: CharactersApi,
    my_api: MyCharacterApi,
    maps: Maps,
    resources: Resources,
    items: Items,
    monsters: Monsters,
    bank: Bank,
    pub name: String,
    pub cooldown_expiration: Option<DateTime<Utc>>,
}

impl Character {
    pub fn new(account: &Account, name: &str) -> Character {
        let mut char = Character {
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
            items: Items::new(account),
            resources: Resources::new(account),
            monsters: Monsters::new(account),
            bank: Bank::new(account),
            name: name.to_owned(),
            cooldown_expiration: None,
        };
        char.cooldown_expiration = char.cooldown_expiration();
        char
    }

    pub fn move_to(&mut self, x: i32, y: i32) -> bool {
        if self.coordinates() == (x, y) {
            return true;
        }
        self.cooldown();
        match self.my_api.move_to(&self.name, x, y) {
            Ok(res) => {
                println!(
                    "{}: moved to {},{} ({})",
                    self.name, x, y, res.data.destination.name
                );
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
                return true;
            }
            Err(ref e) => println!("{}: error while moving: {}", self.name, e),
        }
        false
    }

    fn move_to_bank(&mut self) {
        let _ = self.move_to(4, 1);
    }

    pub fn fight(
        &mut self,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        self.cooldown();
        let res = self.my_api.fight(&self.name);
        match res {
            Ok(ref res) => {
                println!("{} fought and {:?}", self.name, res.data.fight.result);
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!("{}: error during fight: {}", self.name, e),
        };
        res
    }

    pub fn gather(
        &mut self,
    ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>> {
        self.cooldown();
        let res = self.my_api.gather(&self.name);
        match res {
            Ok(ref res) => {
                println!("{}: gathered {:?}", self.name, res.data.details.items);
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!("{}: error during gathering: {}", self.name, e),
        };
        res
    }

    pub fn craft(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        self.cooldown();
        let res = self.my_api.craft(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: crafted {}, {}", self.name, quantity, code);
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!("{}: error during crafting: {}", self.name, e),
        };
        res
    }

    pub fn deposit(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        self.cooldown();
        let res = self.my_api.deposit(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: deposited {} * {}", self.name, code, quantity);
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!(
                "{}: error while depositing {} * {}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    pub fn equip(
        &mut self,
        code: &str,
        slot: equip_schema::Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        self.cooldown();
        let res = self.my_api.equip(&self.name, code, slot, None);
        match res {
            Ok(ref res) => {
                println!(
                    "{}: equiped {} in {:?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!("{}: error while unequiping: {}", self.name, e),
        }
        res
    }

    pub fn unequip(
        &mut self,
        slot: unequip_schema::Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        self.cooldown();
        let res = self.my_api.unequip(&self.name, slot, None);
        match res {
            Ok(ref res) => {
                println!(
                    "{}: unequiped {} from {:?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!("{}: error while unequiping: {}", self.name, e),
        }
        res
    }

    pub fn withdraw(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        self.cooldown();
        let res = self.my_api.withdraw(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: withdrawed {} {}", self.name, code, quantity);
                self.cooldown_expiration =
                    DateTime::parse_from_rfc3339(&res.data.cooldown.expiration)
                        .ok()
                        .map(|d| d.to_utc());
            }
            Err(ref e) => println!(
                "{}: error while withdrawing {} * {}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    pub fn craft_all(&mut self, code: &str) -> bool {
        println!("{} crafting all {}", self.name, code);
        let n = self.has_mats_for(code);
        if n > 0 && self.move_to_craft(code) {
            println!("{} crafted all {} ({})", self.name, code, n);
            return self.craft(code, n).is_ok();
        }
        info!("{} failed to crafted all {} ({})", self.name, code, n);
        false
    }

    fn has_mats_for(&self, code: &str) -> i32 {
        let mut n = 0;
        let mut new_n;

        for mat in self.items.mats_for(code).unwrap() {
            if mat.quantity <= self.amount_in_inventory(&mat.code) {
                new_n = self.amount_in_inventory(&mat.code) / mat.quantity;
                if n == 0 || new_n < n {
                    n = new_n;
                }
            }
        }
        println!("{} has mats to craft {} * {}", self.name, code, n);
        n
    }

    pub fn deposit_all(&mut self) {
        if self.inventory_total() > 0 {
            self.move_to_bank();
            println!("{} depositing all to bank", self.name);
            for i in self.inventory() {
                if i.quantity > 0 {
                    let _ = self.deposit(&i.code, i.quantity);
                }
            }
        }
    }

    fn cooldown(&self) {
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        println!(
            "{}: cooling down for {}.{} secondes",
            self.name,
            s.as_secs(),
            s.subsec_millis()
        );
        sleep(s);
    }

    pub fn inventory(&self) -> Vec<InventorySlot> {
        let char = self.info().unwrap();
        char.data.inventory.unwrap()
    }

    fn info(&self) -> Result<CharacterResponseSchema, Error<GetCharacterCharactersNameGetError>> {
        self.api.get(&self.name)
    }

    pub fn inventory_max_items(&self) -> i32 {
        let char = self.api.get(&self.name).unwrap();
        char.data.inventory_max_items
    }

    pub fn inventory_total(&self) -> i32 {
        let mut i = 0;

        for item in self.inventory() {
            i += item.quantity
        }
        i
    }

    pub fn amount_in_inventory(&self, code: &str) -> i32 {
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

    pub fn inventory_is_full(&self) -> bool {
        self.inventory_total() == self.inventory_max_items()
    }

    pub fn inventory_space_available(&self) -> i32 {
        self.inventory_max_items() - self.inventory_total()
    }

    pub fn weapon_equiped(&self) -> String {
        let char = self.info().unwrap();
        char.data.weapon_slot
    }

    pub fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        match self.api.get(&self.name) {
            Ok(res) => match res.data.cooldown_expiration {
                Some(cd) => match DateTime::parse_from_rfc3339(&cd) {
                    Ok(cd) => Some(cd.to_utc()),
                    Err(_) => None,
                },
                None => None,
            },
            Err(_) => None,
        }
    }

    pub fn remaining_cooldown(&self) -> Duration {
        if let Some(exp) = self.cooldown_expiration {
            let synced = Utc::now() - self.account.server_offset;
            if synced.cmp(&exp.to_utc()) == Ordering::Less {
                return (exp.to_utc() - synced).to_std().unwrap();
            }
        }
        Duration::from_secs(0)
    }

    fn coordinates(&self) -> (i32, i32) {
        let data = self.api.get(&self.name).unwrap().data;
        (data.x, data.y)
    }

    fn level(&self) -> i32 {
        self.api.get(&self.name).unwrap().data.level
    }

    fn skill_level(&self, skill: Skill) -> i32 {
        let data = self.api.get(&self.name).unwrap().data;
        match skill {
            Skill::Cooking => data.cooking_level,
            Skill::Fishing => data.fishing_level,
            Skill::Gearcrafting => data.gearcrafting_level,
            Skill::Jewelrycrafting => data.jewelrycrafting_level,
            Skill::Mining => data.mining_level,
            Skill::Weaponcrafting => data.weaponcrafting_level,
            Skill::Woodcutting => data.woodcutting_level,
        }
    }

    fn closest_map_among(&self, maps: Vec<MapSchema>) -> Option<MapSchema> {
        let (x, y) = self.coordinates();
        self.maps.closest_from_amoung(x, y, maps)
    }

    // fn closest_map_dropping(&self, code: &str) -> Option<(i32, i32)> {
    //     let (mut x, mut y): (i32, i32) = (0, 0);

    //     if let Some(resources) = self.resources.dropping(code) {
    //         for r in resources {
    //             (x, y) = self.closest_map_with_resource(&r).unwrap();
    //         }
    //         return Some((x, y));
    //     }
    //     None
    // }

    fn closest_map_with_resource(&self, code: &str) -> Option<(i32, i32)> {
        if let Ok(maps) = self.maps.with_ressource(code) {
            let map = self.closest_map_among(maps.data).unwrap();
            return Some((map.x, map.y));
        }
        None
    }

    // pub fn fight_until_unsuccessful(&self, x: i32, y: i32) {
    //     let _ = self.move_to(x, y);
    //     loop {
    //         if let Err(Error::ResponseError(res)) = self.fight() {
    //             if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
    //                 println!("{}: needs to cooldown", self.name);
    //                 self.cool_down(self.remaining_cooldown());
    //             }
    //             if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
    //                 println!("{}: inventory is full", self.name);
    //                 self.move_to_bank();
    //                 self.deposit_all();
    //                 let _ = self.move_to(x, y);
    //             }
    //         }
    //     }
    // }

    pub fn craft_all_repeat(&mut self, code: &str) {
        self.cooldown();
        loop {
            self.deposit_all();
            let required_items = self.items.mats_for(code).unwrap();
            for i in required_items {
                let _ = self.withdraw(&i.code, self.inventory_max_items());
            }
            let _ = self.craft_all(code);
        }
    }

    fn move_to_craft(&mut self, code: &str) -> bool {
        let skill = self.items.skill_to_craft(code);
        println!(
            "{}: moving to craft {}: skill found {:?}",
            self.name, code, skill
        );
        match skill {
            Some(Skill::Weaponcrafting) => self.move_to(2, 1),
            Some(Skill::Gearcrafting) => self.move_to(2, 2),
            Some(Skill::Jewelrycrafting) => self.move_to(1, 3),
            Some(Skill::Cooking) => self.move_to(1, 1),
            Some(Skill::Woodcutting) => self.move_to(-2, -3),
            Some(Skill::Mining) => self.move_to(1, 5),
            _ => false,
        }
    }

    pub fn weapons_upgrades(&self) -> Option<Vec<ItemSchema>> {
        let equiped_weapon = self.equipment_in(Slot::Weapon);
        let min_level = equiped_weapon.map(|equiped_weapon| equiped_weapon.item.level);
        match self.items.api.all(
            min_level,
            Some(self.level()),
            None,
            Some(&Type::Weapon.to_string()),
            None,
            None,
            None,
            None,
        ) {
            Ok(items) => Some(items.data),
            Err(_) => None,
        }
    }

    pub fn equipment_in(&self, slot: Slot) -> Option<Box<SingleItemSchema>> {
        let data = self.info().unwrap().data;
        let code = match slot {
            Slot::Weapon => data.weapon_slot,
            Slot::Shield => data.shield_slot,
            Slot::Helmet => data.helmet_slot,
            Slot::BodyArmor => data.body_armor_slot,
            Slot::LegArmor => data.leg_armor_slot,
            Slot::Boots => data.boots_slot,
            Slot::Ring1 => data.ring1_slot,
            Slot::Ring2 => data.ring2_slot,
            Slot::Amulet => data.amulet_slot,
            Slot::Artifact1 => data.artifact1_slot,
            Slot::Artifact2 => data.artifact2_slot,
            Slot::Artifact3 => data.artifact3_slot,
            Slot::Consumable1 => data.consumable1_slot,
            Slot::Consumable2 => data.consumable2_slot,
        };
        match self.items.api.info(&code) {
            Ok(code) => Some(code.data),
            Err(_) => None,
        }
    }

    pub fn weapon_damage(&self) -> i32 {
        match &self.equipment_in(Slot::Weapon) {
            Some(weapon) => self.items.damages(&weapon.item.code),
            None => 0,
        }
    }

    pub fn weapon_upgrade_in_bank(&self) -> Option<String> {
        self.weapons_upgrades()?
            .iter()
            .find(|weapon| {
                self.bank.has_item(&weapon.code).is_some()
                    && self.weapon_damage() < self.items.damages(&weapon.code)
            })
            .map(|weapon| weapon.code.clone())
    }

    pub fn improve_weapon(&mut self) {
        if let Some(code) = self.weapon_upgrade_in_bank() {
            self.move_to_bank();
            if let Some(equiped_weapon) = &self.equipment_in(Slot::Weapon) {
                let _ = self.unequip(unequip_schema::Slot::Weapon);
                let _ = self.deposit(&equiped_weapon.item.code, 1);
            }
            let _ = self.withdraw(&code, 1);
            let _ = self.equip(&code, equip_schema::Slot::Weapon);
        }
    }

    pub fn run(&mut self, role: Role) {
        if Role::Fighter != role
            && self
                .equipment_in(Slot::Weapon)
                .is_some_and(|w| w.item.code == "wooden_stick")
        {
            let _ = self.unequip(unequip_schema::Slot::Weapon);
            self.deposit_all();
        };
        loop {
            if self.inventory_is_full() {
                self.deposit_all();
            }
            match role {
                Role::Fighter => {
                    self.fighter_routin();
                }
                Role::Miner => {
                    self.miner_routin();
                }
                Role::Woodcutter => {
                    self.woodcutter_routin();
                }
                Role::Fisher => {
                    self.fisher_routin();
                }
                Role::Weaponcrafter => {
                    self.weaponcraft_routin();
                }
                Role::Idle => {
                    return;
                }
            };
        }
    }

    fn fighter_routin(&mut self) {
        self.improve_weapon();
        let monster = self.monsters.lower_providing_exp(self.level()).unwrap();
        let (x, y) = self.closest_map_with_resource(&monster.code).unwrap();
        if self.move_to(x, y) {
            let _ = self.fight();
        }
    }

    fn weaponcraft_routin(&mut self) {
        let items = self
            .items
            .best_craftable_at_level(
                self.skill_level(Skill::Weaponcrafting),
                Skill::Weaponcrafting,
            )
            .unwrap();
        for item in &items {
            self.withdraw_max_mats_for(&item.code);
            let _ = self.craft_all(&item.code);
            self.deposit_all();
        }
    }

    fn levelup_skill(&mut self, skill: Skill) -> bool {
        let items = self
            .items
            .best_craftable_at_level(self.skill_level(skill), skill)
            .unwrap();
        if !items.is_empty() && items.iter().any(|i| self.bank.has_mats_for(&i.code) > 0) {
            for item in &items {
                if self.bank.has_mats_for(&item.code) > 0 {
                    self.deposit_all();
                    if self.withdraw_max_mats_for(&item.code) {
                        let _ = self.craft_all(&item.code);
                        self.deposit_all();
                    }
                    return true;
                }
            }
        }
        false
    }

    fn fisher_routin(&mut self) {
        if !self.levelup_skill(Skill::Cooking) {
            self.gather_best_ressource_for(Skill::Fishing);
        }
    }

    fn woodcutter_routin(&mut self) {
        let resource = self
            .resources
            .below_or_equal(self.skill_level(Skill::Woodcutting), Skill::Woodcutting)
            .unwrap();
        let (x, y) = self.closest_map_with_resource(&resource.code).unwrap();
        if self.move_to(x, y) {
            let _ = self.gather();
        }
    }

    fn miner_routin(&mut self) {
        if !self.levelup_skill(Skill::Mining) {
            self.gather_best_ressource_for(Skill::Mining);
        }
    }

    fn gather_best_ressource_for(&mut self, skill: Skill) -> bool {
        let resource = self
            .resources
            .below_or_equal(self.skill_level(skill), skill)
            .unwrap();
        let (x, y) = self.closest_map_with_resource(&resource.code).unwrap();
        if self.move_to(x, y) {
            let _ = self.gather();
            return true;
        }
        false
    }

    fn withdraw_mats_for(&mut self, code: &str, quantity: i32) -> bool {
        println!(
            "{}: withdrawing mats for {} * {}",
            self.name, code, quantity
        );
        let mats = self.items.mats_for(code).unwrap();
        for mat in &mats {
            if self.bank.has_item(&mat.code).unwrap().quantity < mat.quantity * quantity {
                warn!("not enough resources in bank to withdraw the materials required to craft [{code}] * {quantity}");
                return false;
            }
        }
        for mat in &mats {
            let _ = self.withdraw(&mat.code, mat.quantity * quantity);
        }
        true
    }

    /// .withdraw the maximum available amount of mats used to craft the item `code`
    fn withdraw_max_mats_for(&mut self, code: &str) -> bool {
        println!(
            "{}: getting maximum amount of mats in bank to craft {}",
            self.name, code
        );
        let n = self.items.mats_quantity_for(code);
        let can_carry = self.inventory_space_available() / n;
        let total_craftable = self.bank.has_mats_for(code);
        let max = if total_craftable < can_carry {
            total_craftable
        } else {
            can_carry
        };
        self.withdraw_mats_for(code, max)
    }
}

#[derive(PartialEq)]
pub enum Role {
    Fighter,
    Miner,
    Woodcutter,
    Fisher,
    Weaponcrafter,
    Idle,
}

pub enum Action {
    Fight,
    Gather,
    Craft,
    Withdraw,
    Deposit,
}

pub struct Order {}
