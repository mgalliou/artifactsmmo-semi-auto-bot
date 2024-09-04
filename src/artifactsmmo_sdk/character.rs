use super::{
    account::Account,
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    bank::Bank,
    char_config::CharConfig,
    items::{Items, Type},
    maps::Maps,
    monsters::Monsters,
    resources::Resources,
    skill::Skill,
};
use artifactsmmo_openapi::{
    apis::{
        my_characters_api::{
            ActionAcceptNewTaskMyNameActionTaskNewPostError,
            ActionCompleteTaskMyNameActionTaskCompletePostError,
            ActionCraftingMyNameActionCraftingPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionEquipItemMyNameActionEquipPostError, ActionFightMyNameActionFightPostError,
            ActionGatheringMyNameActionGatheringPostError,
            ActionRecyclingMyNameActionRecyclingPostError,
            ActionUnequipItemMyNameActionUnequipPostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        equip_schema::{self, Slot},
        unequip_schema, BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterSchema, EquipmentResponseSchema, ItemSchema, MapSchema, MonsterSchema,
        RecyclingResponseSchema, SingleItemSchema, SkillResponseSchema, TaskResponseSchema,
        TaskRewardResponseSchema,
    },
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{info, warn};
use std::{
    cmp::Ordering, option::Option, sync::{Arc, RwLock}, thread::{self, sleep, JoinHandle}, time::Duration, vec::Vec
};

pub struct Character {
    pub name: String,
    pub info: CharacterSchema,
    my_api: MyCharacterApi,
    account: Account,
    maps: Arc<Maps>,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    items: Arc<Items>,
    bank: Arc<RwLock<Bank>>,
    conf: CharConfig,
}

impl Character {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account: &Account,
        name: &str,
        maps: Arc<Maps>,
        resources: Arc<Resources>,
        monsters: Arc<Monsters>,
        items: Arc<Items>,
        bank: Arc<RwLock<Bank>>,
        conf: CharConfig,
    ) -> Character {
        let api = CharactersApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
        Character {
            name: name.to_owned(),
            info: *api.get(name).unwrap().data,
            my_api: MyCharacterApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            account: account.clone(),
            maps,
            resources,
            monsters,
            items,
            bank,
            conf,
        }
    }

    pub fn run(mut char: Character) -> Result<JoinHandle<()>, std::io::Error> {
     thread::Builder::new()
            .name(char.name.to_string())
            .spawn(move || {
                char.run2();
            })
    }

    pub fn run2(&mut self) {
        if Role::Fighter != self.conf.role
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
            if self.info.task.is_empty() || self.task_finished() {
                self.move_to(1, 2);
                if self.task_finished() {
                    let _ = self.complete_task();
                }
                let _ = self.accept_task();
            }
            match self.conf.role {
                Role::Fighter => {
                    self.fighter_routin();
                }
                Role::Miner | Role::Woodcutter => {
                    self.gatherer_routin();
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
        if let Some(skill) = self.target_skill() {
            if self.levelup_by_crafting(skill) {
                return;
            }
        };
        if let Some(monster) = self.target_monster() {
            self.improve_weapon();
            self.kill_monster(&monster.code);
        }
    }

    fn kill_monster(&mut self, code: &str) {
        if let Some((x, y)) = self.closest_map_with_resource(code) {
            if self.move_to(x, y) {
                let _ = self.fight();
            }
        }
    }

    fn target_skill(&mut self) -> Option<Skill> {
        let mut skills = vec![];
        if self.conf.weaponcraft && self.conf.level_weaponcraft {
            skills.push(Skill::Weaponcrafting);
        }
        if self.conf.gearcraft && self.conf.level_gearcraft {
            skills.push(Skill::Gearcrafting);
        }
        if self.conf.jewelcraft && self.conf.level_jewelcraft {
            skills.push(Skill::Jewelrycrafting);
        }
        if self.conf.cook && self.conf.level_cook {
            skills.push(Skill::Cooking);
        }
        skills.sort_by_key(|s| self.skill_level(*s));
        skills.into_iter().find(|&skill| self
                .items
                .providing_exp(self.skill_level(skill), skill)
                .iter()
                .filter(|i| !self.items.is_crafted_with(&i.code, "jasper_crystal"))
                .any(|i| self.bank.read().is_ok_and(|b| b.has_mats_for(&i.code) > 0)))
    }

    // TODO: ruduce amount of cloned() calls if possible
    fn target_monster(&mut self) -> Option<MonsterSchema> {
        if self.conf.do_tasks && self.info.task_type == "monsters" && !self.task_finished() {
            self.monsters.get(&self.info.task).cloned()
        } else if let Some(monster) = self.conf.fight_target.clone() {
            self.monsters.get(&monster).cloned()
        } else {
            self.monsters.lowest_providing_exp(self.info.level).cloned()
        }
    }

    fn task_finished(&mut self) -> bool {
        self.info.task_progress >= self.info.task_total
    }

    fn gatherer_routin(&mut self) {
        if let Some(code) = self.conf.resource.clone() {
            let processed = self.items.with_material(&code);
            if !processed
                .iter()
                .any(|i| self.conf.craft_from_bank && self.craft_all_from_bank(&i.code))
            {
                self.gather_resource(&code);
                if self.inventory_is_full() && self.conf.process_gathered {
                    processed.iter().for_each(|i| {
                        self.craft_all(&i.code);
                        self.deposit_all();
                    });
                }
            }
        } else if let Some(skill) = self.conf.role.to_skill() {
            self.levelup_by_gathering(skill);
        }
    }

    fn fisher_routin(&mut self) {
        if !self.levelup_by_crafting(Skill::Cooking) {
            self.levelup_by_gathering(Skill::Fishing);
        }
    }

    fn weaponcraft_routin(&mut self) {
        self.levelup_by_crafting(Skill::Weaponcrafting);
    }

    fn equipment_in(&self, slot: Slot) -> Option<SingleItemSchema> {
        let code = match slot {
            Slot::Weapon => &self.info.weapon_slot,
            Slot::Shield => &self.info.shield_slot,
            Slot::Helmet => &self.info.helmet_slot,
            Slot::BodyArmor => &self.info.body_armor_slot,
            Slot::LegArmor => &self.info.leg_armor_slot,
            Slot::Boots => &self.info.boots_slot,
            Slot::Ring1 => &self.info.ring1_slot,
            Slot::Ring2 => &self.info.ring2_slot,
            Slot::Amulet => &self.info.amulet_slot,
            Slot::Artifact1 => &self.info.artifact1_slot,
            Slot::Artifact2 => &self.info.artifact2_slot,
            Slot::Artifact3 => &self.info.artifact3_slot,
            Slot::Consumable1 => &self.info.consumable1_slot,
            Slot::Consumable2 => &self.info.consumable2_slot,
        };
        self.items.api.info(code).ok().map(|i| *i.data)
    }

    fn levelup_by_crafting(&mut self, skill: Skill) -> bool {
        matches!(self.items.best_for_leveling(self.skill_level(skill), skill), Some(item) if {
            if self
                .bank
                .read()
                .is_ok_and(|b| b.has_mats_for(&item.code) > 0)
            {
                return self.craft_all_from_bank(&item.code);
            };
            false
        })
    }

    //fn levelup_by_crafting(&mut self, skill: Skill) -> bool {
    //    let items = self
    //        .items
    //        .best_for_leveling(self.skill_level(skill), skill)
    //        .unwrap();
    //    if !items.is_empty()
    //        && items
    //            .iter()
    //            .any(|i| self.bank.read().is_ok_and(|b| b.has_mats_for(&i.code) > 0))
    //    {
    //        return items.iter().any(|i| self.craft_all_from_bank(&i.code));
    //    }
    //    false
    //}

    fn craft_all_from_bank(&mut self, code: &str) -> bool {
        if self.bank.read().is_ok_and(|b| b.has_mats_for(code) > 0) {
            self.deposit_all();
            if self.withdraw_max_mats_for(code) {
                let _ = self.craft_all(code);
                self.deposit_all();
            }
            return true;
        }
        false
    }

    fn levelup_by_gathering(&mut self, skill: Skill) -> bool {
        let resource = self
            .resources
            .lowest_providing_exp(self.skill_level(skill), skill)
            .unwrap()
            .clone();
        self.gather_resource(&resource.code)
    }

    fn gather_resource(&mut self, code: &str) -> bool {
        if let Some(map) = self.closest_map_dropping(code) {
            self.move_to(map.x, map.y) && self.gather().is_ok()
        } else {
            false
        }
    }

    fn skill_level(&self, skill: Skill) -> i32 {
        match skill {
            Skill::Cooking => self.info.cooking_level,
            Skill::Fishing => self.info.fishing_level,
            Skill::Gearcrafting => self.info.gearcrafting_level,
            Skill::Jewelrycrafting => self.info.jewelrycrafting_level,
            Skill::Mining => self.info.mining_level,
            Skill::Weaponcrafting => self.info.weaponcrafting_level,
            Skill::Woodcutting => self.info.woodcutting_level,
        }
    }

    fn deposit_all(&mut self) {
        if self.inventory_total() > 0 {
            self.move_to_bank();
            println!("{} depositing all to bank", self.name);
            if let Some(inventory) = self.info.inventory.clone() {
                for i in &inventory {
                    if i.quantity > 0 {
                        let _ = self.deposit(&i.code, i.quantity);
                    }
                }
            }
        }
    }

    fn withdraw_mats_for(&mut self, code: &str, quantity: i32) -> bool {
        println!(
            "{}: withdrawing mats for {} * {}",
            self.name, code, quantity
        );
        let mats = self.items.mats_for(code).unwrap();
        for mat in &mats {
            if !self
                .bank
                .read()
                .is_ok_and(|b| b.has_item(&mat.code).unwrap().quantity >= mat.quantity * quantity)
            {
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
        self.move_to_bank();
        println!(
            "{}: getting maximum amount of mats in bank to craft {}",
            self.name, code
        );
        let can_carry = self.inventory_free_space() / self.items.mats_quantity_for(code);
        let can_craft_from_bank = self.bank.read().map_or(0, |b| b.has_mats_for(code));
        let max = if can_craft_from_bank < can_carry {
            can_craft_from_bank
        } else {
            can_carry
        };
        self.withdraw_mats_for(code, max)
    }

    fn craft_all(&mut self, code: &str) -> bool {
        println!("{}: crafting all {}", self.name, code);
        let n = self.has_mats_for(code);
        if n > 0 && self.move_to_craft(code) && self.craft(code, n).is_ok() {
            println!("{} crafted all {} ({})", self.name, code, n);
            return true;
        }
        info!("{} failed to crafted all {} ({})", self.name, code, n);
        false
    }

    fn move_to_bank(&mut self) {
        let _ = self.move_to(4, 1);
    }

    fn move_to(&mut self, x: i32, y: i32) -> bool {
        if (self.info.x, self.info.y) == (x, y) {
            return true;
        }
        self.wait_for_cooldown();
        match self.my_api.move_to(&self.name, x, y) {
            Ok(res) => {
                println!(
                    "{}: moved to {},{} ({})",
                    self.name, x, y, res.data.destination.name
                );
                self.info = *res.data.character.clone();
                return true;
            }
            Err(ref e) => println!("{}: error while moving: {}", self.name, e),
        }
        false
    }

    fn fight(
        &mut self,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.fight(&self.name);
        match res {
            Ok(ref res) => {
                println!("{} fought and {:?}", self.name, res.data.fight.result);
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error during fight: {}", self.name, e),
        };
        res
    }

    fn gather(
        &mut self,
    ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.gather(&self.name);
        match res {
            Ok(ref res) => {
                print!("{}: gathered: ", self.name);
                for item in &res.data.details.items {
                    print!("{} * {},", item.code, item.quantity);
                }
                println!();
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error during gathering: {}", self.name, e),
        };
        res
    }

    fn withdraw(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        self.wait_for_cooldown();
        let res = self.my_api.withdraw(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: withdrawed {} {}", self.name, code, quantity);
                self.info = *res.data.character.clone();
                let _ = self
                    .bank
                    .write()
                    .map(|mut bank| bank.content = res.data.bank.clone());
            }
            Err(ref e) => println!(
                "{}: error while withdrawing {} * {}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    fn deposit(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        self.wait_for_cooldown();
        let res = self.my_api.deposit(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: deposited {} * {}", self.name, code, quantity);
                self.info = *res.data.character.clone();
                let _ = self
                    .bank
                    .write()
                    .map(|mut bank| bank.content = res.data.bank.clone());
            }
            Err(ref e) => println!(
                "{}: error while depositing {} * {}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    fn craft(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.craft(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: crafted {}, {}", self.name, quantity, code);
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error during crafting: {}", self.name, e),
        };
        res
    }

    fn recycle(
        &mut self,
        code: &str,
        quantity: i32,
    ) -> Result<RecyclingResponseSchema, Error<ActionRecyclingMyNameActionRecyclingPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.recycle(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                println!("{}: recycled {}, {}", self.name, quantity, code);
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error during crafting: {}", self.name, e),
        };
        res
    }

    fn equip(
        &mut self,
        code: &str,
        slot: equip_schema::Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.equip(&self.name, code, slot, None);
        match res {
            Ok(ref res) => {
                println!(
                    "{}: equiped {} in {:?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error while unequiping: {}", self.name, e),
        }
        res
    }

    fn unequip(
        &mut self,
        slot: unequip_schema::Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.unequip(&self.name, slot, None);
        match res {
            Ok(ref res) => {
                println!(
                    "{}: unequiped {} from {:?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error while unequiping: {}", self.name, e),
        }
        res
    }

    fn accept_task(
        &mut self,
    ) -> Result<TaskResponseSchema, Error<ActionAcceptNewTaskMyNameActionTaskNewPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.accept_task(&self.name);
        match res {
            Ok(ref res) => {
                println!("{}: accepted new task: {:?}", self.name, res.data.task);
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error while accepting: {}", self.name, e),
        }
        res
    }

    fn complete_task(
        &mut self,
    ) -> Result<TaskRewardResponseSchema, Error<ActionCompleteTaskMyNameActionTaskCompletePostError>>
    {
        self.wait_for_cooldown();
        let res = self.my_api.complete_task(&self.name);
        match res {
            Ok(ref res) => {
                println!("{}: completed task: {:?}", self.name, res.data.reward);
                self.info = *res.data.character.clone();
            }
            Err(ref e) => println!("{}: error while accepting: {}", self.name, e),
        }
        res
    }

    fn wait_for_cooldown(&self) {
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

    fn remaining_cooldown(&self) -> Duration {
        if let Some(exp) = self.cooldown_expiration() {
            let synced = Utc::now() - self.account.server_offset;
            if synced.cmp(&exp.to_utc()) == Ordering::Less {
                return (exp.to_utc() - synced).to_std().unwrap();
            }
        }
        Duration::from_secs(0)
    }

    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.info
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }

    fn inventory_is_full(&self) -> bool {
        self.inventory_total() == self.info.inventory_max_items
    }

    fn amount_in_inventory(&self, code: &str) -> i32 {
        self.info
            .inventory
            .as_ref()
            .map(|inv| {
                inv.iter()
                    .filter(|i| i.code == code)
                    .map(|i| i.quantity)
                    .sum()
            })
            .unwrap_or(0)
    }

    fn inventory_free_space(&self) -> i32 {
        self.info.inventory_max_items - self.inventory_total()
    }

    fn inventory_total(&self) -> i32 {
        self.info
            .inventory
            .as_ref()
            .map_or(0, |inv| inv.iter().map(|i| i.quantity).sum())
    }

    fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats_for(code)
            .and_then(|mats| {
                mats.iter()
                    .filter(|mat| mat.quantity > 0)
                    .map(|mat| self.amount_in_inventory(&mat.code) / mat.quantity)
                    .max()
            })
            .unwrap_or(0)
    }

    fn closest_map_among<'a>(&'a self, maps: Vec<&'a MapSchema>) -> Option<&MapSchema> {
        Maps::closest_from_amoung(self.info.x, self.info.y, maps)
    }

    fn closest_map_dropping(&self, code: &str) -> Option<&MapSchema> {
        match self.resources.dropping(code) {
            Some(resources) => {
                let maps = self
                    .maps
                    .data
                    .iter()
                    .filter(|m| Maps::has_one_of_resource(m, resources.clone()))
                    .collect_vec();
                Maps::closest_from_amoung(self.info.x, self.info.y, maps)
            }
            _ => None,
        }
    }

    fn closest_map_with_resource(&self, code: &str) -> Option<(i32, i32)> {
        self.maps
            .with_ressource(code)
            .and_then(|maps| self.closest_map_among(maps))
            .map(|map| (map.x, map.y))
    }

    fn move_to_craft(&mut self, code: &str) -> bool {
        let skill = self.items.skill_to_craft(code);
        println!(
            "{}: moving to craft {}: skill found {:?}",
            self.name, code, skill
        );
        match skill {
            Some(Skill::Weaponcrafting) => self.move_to(2, 1),
            Some(Skill::Gearcrafting) => self.move_to(3, 1),
            Some(Skill::Jewelrycrafting) => self.move_to(1, 3),
            Some(Skill::Cooking) => self.move_to(1, 1),
            Some(Skill::Woodcutting) => self.move_to(-2, -3),
            Some(Skill::Mining) => self.move_to(1, 5),
            _ => false,
        }
    }

    fn weapon_damage(&self) -> i32 {
        self.equipment_in(Slot::Weapon)
            .map(|w| self.items.damages(&w.item.code))
            .unwrap_or(0)
    }

    fn slot_to_type(slot: Slot) -> Type {
        match slot {
            Slot::Weapon => Type::Weapon,
            Slot::Shield => Type::Shield,
            Slot::Helmet => Type::Helmet,
            Slot::BodyArmor => Type::BodyArmor,
            Slot::LegArmor => Type::LegArmor,
            Slot::Boots => Type::Boots,
            Slot::Ring1 => Type::Ring,
            Slot::Ring2 => Type::Ring,
            Slot::Amulet => Type::Amulet,
            Slot::Artifact1 => Type::Artifact,
            Slot::Artifact2 => Type::Artifact,
            Slot::Artifact3 => Type::Artifact,
            Slot::Consumable1 => Type::Consumable,
            Slot::Consumable2 => Type::Consumable,
        }
    }

    fn improve_equipment(&mut self) {
        self.improve_slot(Slot::Helmet);
        self.improve_slot(Slot::LegArmor);
        self.improve_slot(Slot::BodyArmor);
        self.improve_slot(Slot::Boots);
        self.improve_slot(Slot::Shield);
        self.improve_slot(Slot::Ring1);
        self.improve_slot(Slot::Ring2);
        self.improve_slot(Slot::Amulet);
        self.improve_slot(Slot::Artifact1);
        self.improve_slot(Slot::Artifact2);
        self.improve_slot(Slot::Artifact3);
        self.improve_slot(Slot::Consumable1);
        self.improve_slot(Slot::Consumable2);
    }

    fn improve_slot(&mut self, slot: Slot) {
        match slot {
            Slot::Weapon => todo!(),
            Slot::Shield
            | Slot::Helmet
            | Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Boots
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet => todo!(),
            Slot::Artifact1 | Slot::Artifact2 | Slot::Artifact3 => todo!(),
            Slot::Consumable1 | Slot::Consumable2 => todo!(),
        }
    }

    //fn slot_upgrade(&mut self, slot: Slot

    fn improve_weapon(&mut self) {
        if let Some(code) = self.weapon_upgrade_in_bank() {
            self.move_to_bank();
            if let Some(equiped_weapon) = &self.equipment_in(Slot::Weapon) {
                if self.unequip(unequip_schema::Slot::Weapon).is_ok() {
                    let _ = self.deposit(&equiped_weapon.item.code, 1);
                }
            }
            if self.withdraw(&code, 1).is_ok() {
                let _ = self.equip(&code, equip_schema::Slot::Weapon);
            }
        }
    }

    // fn improve_equipment(&mut self, slot: Slot) {
    //     let upgrades = self.equipment_upgrades(slot);
    //     for item in upgrades.unwrap() {
    //         if self.equipment_in(slot).is_some_and(|i| i.item.code != item.code) {
    //             self.bank.read().is_ok_and(|b| b.has_mats_for(item.code))
    //         }
    //     }

    //     todo!()
    // }

    fn weapon_upgrade_in_bank(&self) -> Option<String> {
        self.equipment_upgrades(Slot::Weapon)?
            .iter()
            .find(|weapon| {
                self.bank
                    .read()
                    .is_ok_and(|b| b.has_item(&weapon.code).is_some())
                    && self.weapon_damage() < self.items.damages(&weapon.code)
            })
            .map(|weapon| weapon.code.clone())
    }

    /// return all the items for the given slot between the equiped item level
    /// and the character level
    fn equipment_upgrades(&self, slot: Slot) -> Option<Vec<ItemSchema>> {
        let min_level = self.equipment_in(slot).map(|e| e.item.level);
        self.items
            .api
            .all(
                min_level,
                Some(self.info.level),
                None,
                Some(&Character::slot_to_type(slot).to_string()),
                None,
                None,
            )
            .ok()
    }

    // fn fight_until_unsuccessful(&self, x: i32, y: i32) {
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

    // fn craft_all_repeat(&mut self, code: &str) {
    //     self.wait_for_cooldown();
    //     loop {
    //         self.deposit_all();
    //         let required_items = self.items.mats_for(code).unwrap();
    //         for i in required_items {
    //             let _ = self.withdraw(&i.code, self.info.inventory_max_items);
    //         }
    //         let _ = self.craft_all(code);
    //     }
    // }
}

#[derive(PartialEq, Default)]
pub enum Role {
    Fighter,
    Miner,
    Woodcutter,
    Fisher,
    Weaponcrafter,
    #[default]
    Idle,
}

impl Role {
    pub fn to_skill(&self) -> Option<Skill> {
        match *self {
            Role::Fighter => None,
            Role::Miner => Some(Skill::Mining),
            Role::Woodcutter => Some(Skill::Woodcutting),
            Role::Fisher => Some(Skill::Fishing),
            Role::Weaponcrafter => Some(Skill::Weaponcrafting),
            Role::Idle => None,
        }
    }
}

pub enum Action {
    Fight,
    Gather,
    Craft,
    Withdraw,
    Deposit,
}

pub struct Order {}
