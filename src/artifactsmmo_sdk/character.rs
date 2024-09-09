use super::{
    account::Account,
    api::my_character::MyCharacterApi,
    bank::Bank,
    char_config::CharConfig,
    items::{Items, Type},
    maps::Maps,
    monsters::Monsters,
    resources::Resources,
    skill::Skill,
    ItemSchemaExt, MapSchemaExt,
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
            ActionTaskCancelMyNameActionTaskCancelPostError,
            ActionUnequipItemMyNameActionUnequipPostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        equip_schema::{self, Slot},
        unequip_schema, BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterSchema, EquipmentResponseSchema, InventorySlot, ItemSchema, MapSchema,
        MonsterSchema, RecyclingResponseSchema, ResourceSchema, SkillResponseSchema,
        TaskCancelledResponseSchema, TaskResponseSchema, TaskRewardResponseSchema,
    },
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    cmp::Ordering,
    io,
    option::Option,
    sync::{Arc, RwLock},
    thread::{self, sleep, JoinHandle},
    time::Duration,
    vec::Vec,
};

pub struct Character {
    name: String,
    my_api: MyCharacterApi,
    account: Account,
    maps: Arc<Maps>,
    resources: Arc<Resources>,
    monsters: Arc<Monsters>,
    items: Arc<Items>,
    bank: Arc<RwLock<Bank>>,
    pub conf: Arc<RwLock<CharConfig>>,
    pub data: Arc<RwLock<CharacterSchema>>,
}

impl Character {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account: &Account,
        maps: Arc<Maps>,
        resources: Arc<Resources>,
        monsters: Arc<Monsters>,
        items: Arc<Items>,
        bank: Arc<RwLock<Bank>>,
        conf: Arc<RwLock<CharConfig>>,
        data: Arc<RwLock<CharacterSchema>>,
    ) -> Character {
        Character {
            name: data.read().map(|d| d.name.to_owned()).unwrap(),
            conf,
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
            data,
        }
    }

    pub fn run(char: Character) -> Result<JoinHandle<()>, io::Error> {
        thread::Builder::new()
            .name(char.data().name.to_owned())
            .spawn(move || {
                char.run2();
            })
    }

    fn run2(&self) {
        info!("{}: started !", self.name);
        if Role::Fighter != self.conf().role
            && self
                .equipment_in(Slot::Weapon)
                .is_some_and(|w| w.code == "wooden_stick")
        {
            let _ = self.action_unequip(unequip_schema::Slot::Weapon);
            self.deposit_all();
        };
        loop {
            self.process_inventory();
            self.process_task();
            if let Some(skill) = self.target_skill() {
                if self.levelup_by_crafting(skill) {
                    return;
                }
            }
            if let Some(monster) = self.target_monster().cloned() {
                self.improve_weapon();
                self.kill_monster(&monster.code);
            } else if let Some(resource) = self.target_resource().cloned() {
                // TODO: Improve this
                if let Some(item) = &self.conf().target_item {
                    if !self
                        .items
                        .crafted_with(item)
                        .into_iter()
                        .cloned()
                        .collect_vec()
                        .into_iter()
                        .any(|i| self.conf().craft_from_bank && self.craft_all_from_bank(&i.code))
                    {
                        self.gather_resource(&resource.code);
                    }
                }
            }
        }
    }

    fn conf(&self) -> CharConfig {
        self.conf.read().unwrap().clone()
    }

    fn data(&self) -> CharacterSchema {
        self.data.read().unwrap().clone()
    }

    fn process_inventory(&self) {
        if self.inventory_is_full() {
            if self.conf().process_gathered {
                self.process_raw_mats();
            }
            self.deposit_all_mats();
        }
    }

    fn process_task(&self) {
        if self.data().task.is_empty() || self.task_finished() {
            if self.task_finished() {
                let _ = self.action_complete_task();
            }
            let _ = self.action_accept_task();
        }
    }

    fn process_raw_mats(&self) {
        self.inventory_raw_mats()
            .iter()
            .filter_map(|rm| {
                self.items
                    .crafted_with(&rm.code)
                    .into_iter()
                    .filter(|cw| self.has_mats_for(&cw.code) > 0)
                    .max_by_key(|cw| cw.level)
            })
            .cloned()
            .collect_vec()
            .iter()
            .for_each(|p| {
                self.craft_all(&p.code);
            });
    }

    fn inventory_raw_mats(&self) -> Vec<&ItemSchema> {
        self.data()
            .inventory
            .iter()
            .flatten()
            .filter_map(|slot| self.items.get(&slot.code))
            .filter(|i| i.is_raw_mat())
            .collect_vec()
    }

    fn kill_monster(&self, code: &str) -> bool {
        if let Some(map) = self.closest_map_with_resource(code) {
            return self.action_move(map.x, map.y) && self.action_fight().is_ok();
        }
        false
    }

    fn gather_resource(&self, code: &str) -> bool {
        if let Some(map) = self.closest_map_with_resource(code) {
            return self.action_move(map.x, map.y) && self.action_gather().is_ok();
        }
        false
    }

    fn target_skill(&self) -> Option<Skill> {
        let mut skills = vec![];
        if self.conf().weaponcraft && self.conf().level_weaponcraft {
            skills.push(Skill::Weaponcrafting);
        }
        if self.conf().gearcraft && self.conf().level_gearcraft {
            skills.push(Skill::Gearcrafting);
        }
        if self.conf().jewelcraft && self.conf().level_jewelcraft {
            skills.push(Skill::Jewelrycrafting);
        }
        if self.conf().cook && self.conf().level_cook {
            skills.push(Skill::Cooking);
        }
        skills.sort_by_key(|s| self.skill_level(*s));
        skills.into_iter().find(|&skill| {
            self.items
                .providing_exp(self.skill_level(skill), skill)
                .iter()
                .filter(|i| !i.is_crafted_with("jasper_crystal"))
                .any(|i| self.bank.read().is_ok_and(|b| b.has_mats_for(&i.code) > 0))
        })
    }

    fn target_monster(&self) -> Option<&MonsterSchema> {
        if self.conf().role == Role::Fighter {
            if self.conf().do_tasks && self.data().task_type == "monsters" && !self.task_finished()
            {
                return self.monsters.get(&self.data().task);
            } else if let Some(monster) = &self.conf().fight_target {
                return self.monsters.get(monster);
            } else {
                return self.monsters.lowest_providing_exp(self.data().level);
            }
        }
        None
    }

    fn target_resource(&self) -> Option<&ResourceSchema> {
        match self.conf().role {
            Role::Miner | Role::Woodcutter | Role::Fisher => {
                if let Some(item) = &self.conf().target_item {
                    return self
                        .resources
                        .dropping(item)
                        .iter()
                        .min_by_key(|r| r.level)
                        .copied();
                } else if let Some(skill) = self.conf().role.to_skill() {
                    return self
                        .resources
                        .lowest_providing_exp(self.skill_level(skill), skill);
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn task_finished(&self) -> bool {
        self.data().task_progress >= self.data().task_total
    }

    fn equipment_in(&self, slot: Slot) -> Option<&ItemSchema> {
        let binding = self.data();
        let code = match slot {
            Slot::Weapon => &binding.weapon_slot,
            Slot::Shield => &binding.shield_slot,
            Slot::Helmet => &binding.helmet_slot,
            Slot::BodyArmor => &binding.body_armor_slot,
            Slot::LegArmor => &binding.leg_armor_slot,
            Slot::Boots => &binding.boots_slot,
            Slot::Ring1 => &binding.ring1_slot,
            Slot::Ring2 => &binding.ring2_slot,
            Slot::Amulet => &binding.amulet_slot,
            Slot::Artifact1 => &binding.artifact1_slot,
            Slot::Artifact2 => &binding.artifact2_slot,
            Slot::Artifact3 => &binding.artifact3_slot,
            Slot::Consumable1 => &binding.consumable1_slot,
            Slot::Consumable2 => &binding.consumable2_slot,
        };
        self.items.get(code)
    }

    fn levelup_by_crafting(&self, skill: Skill) -> bool {
        self.items
            .best_for_leveling(self.skill_level(skill), skill)
            .is_some_and(|item| {
                self.bank
                    .read()
                    .is_ok_and(|bank| bank.has_mats_for(&item.code) > 0)
            })
    }

    fn craft_all_from_bank(&self, code: &str) -> bool {
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

    fn skill_level(&self, skill: Skill) -> i32 {
        match skill {
            Skill::Cooking => self.data().cooking_level,
            Skill::Fishing => self.data().fishing_level,
            Skill::Gearcrafting => self.data().gearcrafting_level,
            Skill::Jewelrycrafting => self.data().jewelrycrafting_level,
            Skill::Mining => self.data().mining_level,
            Skill::Weaponcrafting => self.data().weaponcrafting_level,
            Skill::Woodcutting => self.data().woodcutting_level,
        }
    }

    /// Returns a copy of the inventory to be used while depositing or
    /// withdrawing items.
    fn inventory_copy(&self) -> Vec<InventorySlot> {
        self.data()
            .inventory
            .iter()
            .flatten()
            .cloned()
            .collect_vec()
    }

    fn deposit_all_mats(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!(
            "{} is going to depositing all materials to the bank.",
            self.name
        );
        for slot in self.inventory_copy() {
            if slot.quantity > 0 && self.items.is_of_type(&slot.code, Type::Resource) {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    fn deposit_all(&self) {
        if self.inventory_total() <= 0 {
            return;
        }
        info!(
            "{} is going to depositing all items to the bank.",
            self.name
        );
        for slot in self.inventory_copy() {
            if slot.quantity > 0 {
                let _ = self.action_deposit(&slot.code, slot.quantity);
            }
        }
    }

    fn withdraw_mats_for(&self, code: &str, quantity: i32) -> bool {
        info!(
            "{}: withdrawing mats for {} * {}",
            self.name, code, quantity
        );
        let mats = self.items.mats(code);
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
            let _ = self.action_withdraw(&mat.code, mat.quantity * quantity);
        }
        true
    }

    /// .withdraw the maximum available amount of mats used to craft the item `code`
    fn withdraw_max_mats_for(&self, code: &str) -> bool {
        info!(
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

    fn craft_all(&self, code: &str) -> bool {
        info!("{}: crafting all {}", self.name, code);
        let n = self.has_mats_for(code);
        if n > 0 && self.action_craft(code, n).is_ok() {
            info!("{} crafted all {} ({})", self.name, code, n);
            return true;
        }
        error!("{} failed to crafted all {} ({})", self.name, code, n);
        false
    }

    fn move_to_bank(&self) {
        let _ = self.action_move(4, 1);
    }

    fn action_move(&self, x: i32, y: i32) -> bool {
        if (self.data().x, self.data().y) == (x, y) {
            return true;
        }
        self.wait_for_cooldown();
        match self.my_api.move_to(&self.name, x, y) {
            Ok(res) => {
                info!(
                    "{}: moved to {},{} ({})",
                    self.name, x, y, res.data.destination.name
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref())
            }
            Err(ref e) => error!("{}: error while moving: {}", self.name, e),
        }
        false
    }

    fn action_fight(
        &self,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.fight(&self.name);
        match res {
            Ok(ref res) => {
                info!("{} fought and {:?}", self.name, res.data.fight.result);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref())
            }
            Err(ref e) => error!("{}: error during fight: {}", self.name, e),
        };
        res
    }

    fn action_gather(
        &self,
    ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.gather(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}: gathered: {:?}", self.name, res.data.details);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error during gathering: {}", self.name, e),
        };
        res
    }

    fn action_withdraw(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        self.move_to_bank();
        self.wait_for_cooldown();
        let res = self.my_api.withdraw(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!("{}: withdrawed {} {}", self.name, code, quantity);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
                let _ = self
                    .bank
                    .write()
                    .map(|mut bank| bank.content = res.data.bank.clone());
            }
            Err(ref e) => error!(
                "{}: error while withdrawing {} * {}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    fn action_deposit(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        self.move_to_bank();
        self.wait_for_cooldown();
        let res = self.my_api.deposit(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!("{}: deposited {} * {}", self.name, code, quantity);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
                let _ = self
                    .bank
                    .write()
                    .map(|mut bank| bank.content = res.data.bank.clone());
            }
            Err(ref e) => error!(
                "{}: error while depositing {} * {}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    fn action_craft(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        self.move_to_craft(code);
        self.wait_for_cooldown();
        let res = self.my_api.craft(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!("{}: crafted {}, {}", self.name, quantity, code);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error during crafting: {}", self.name, e),
        };
        res
    }

    fn action_recycle(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<RecyclingResponseSchema, Error<ActionRecyclingMyNameActionRecyclingPostError>> {
        self.move_to_craft(code);
        self.wait_for_cooldown();
        let res = self.my_api.recycle(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!("{}: recycled {}, {}", self.name, quantity, code);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error during crafting: {}", self.name, e),
        };
        res
    }

    fn action_equip(
        &self,
        code: &str,
        slot: equip_schema::Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.equip(&self.name, code, slot, None);
        match res {
            Ok(ref res) => {
                info!(
                    "{}: equiped {} in {:?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while unequiping: {}", self.name, e),
        }
        res
    }

    fn action_unequip(
        &self,
        slot: unequip_schema::Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.unequip(&self.name, slot, None);
        match res {
            Ok(ref res) => {
                info!(
                    "{}: unequiped {} from {:?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while unequiping: {}", self.name, e),
        }
        res
    }

    fn action_accept_task(
        &self,
    ) -> Result<TaskResponseSchema, Error<ActionAcceptNewTaskMyNameActionTaskNewPostError>> {
        self.action_move(1, 2);
        self.wait_for_cooldown();
        let res = self.my_api.accept_task(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}: accepted new task: {:?}", self.name, res.data.task);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting: {}", self.name, e),
        }
        res
    }

    fn action_complete_task(
        &self,
    ) -> Result<TaskRewardResponseSchema, Error<ActionCompleteTaskMyNameActionTaskCompletePostError>>
    {
        self.action_move(1, 2);
        self.wait_for_cooldown();
        let res = self.my_api.complete_task(&self.name);
        match res {
            Ok(ref res) => {
                error!("{}: completed task: {:?}", self.name, res.data.reward);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting: {}", self.name, e),
        }
        res
    }

    fn action_cancel_task(
        &self,
    ) -> Result<TaskCancelledResponseSchema, Error<ActionTaskCancelMyNameActionTaskCancelPostError>>
    {
        self.action_move(1, 2);
        self.wait_for_cooldown();
        let res = self.my_api.cancel_task(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}: canceled task: {:?}", self.name, self.data().task);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting: {}", self.name, e),
        }
        res
    }

    fn wait_for_cooldown(&self) {
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        info!(
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
        self.data()
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }

    fn inventory_is_full(&self) -> bool {
        self.inventory_total() == self.data().inventory_max_items
    }

    fn amount_in_inventory(&self, code: &str) -> i32 {
        self.data()
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
        self.data().inventory_max_items - self.inventory_total()
    }

    fn inventory_total(&self) -> i32 {
        self.data()
            .inventory
            .as_ref()
            .map_or(0, |inv| inv.iter().map(|i| i.quantity).sum())
    }

    fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats(code)
            .iter()
            .filter(|mat| mat.quantity > 0)
            .map(|mat| self.amount_in_inventory(&mat.code) / mat.quantity)
            .max()
            .unwrap_or(0)
    }

    fn closest_map_among<'a>(&'a self, maps: Vec<&'a MapSchema>) -> Option<&MapSchema> {
        Maps::closest_from_amoung(self.data().x, self.data().y, maps)
    }

    fn closest_map_dropping(&self, code: &str) -> Option<&MapSchema> {
        let resources = self.resources.dropping(code);
        let maps = self
            .maps
            .data
            .iter()
            .filter(|m| m.has_one_of_resource(&resources))
            .collect_vec();
        Maps::closest_from_amoung(self.data().x, self.data().y, maps)
    }

    fn closest_map_with_resource(&self, code: &str) -> Option<&MapSchema> {
        let maps = self.maps.with_ressource(code);
        if maps.is_empty() {
            return None;
        }
        self.closest_map_among(maps)
    }

    fn move_to_craft(&self, code: &str) -> bool {
        if let Some(dest) = self
            .items
            .skill_to_craft(code)
            .and_then(|s| self.maps.to_craft(s))
        {
            self.action_move(dest.x, dest.y);
        }
        false
    }

    fn weapon_damage(&self) -> i32 {
        self.equipment_in(Slot::Weapon)
            .map(|w| w.damages())
            .unwrap_or(0)
    }

    fn improve_equipment(&self) {
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

    fn improve_slot(&self, slot: Slot) {
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

    fn improve_weapon(&self) {
        if let Some(code) = self.weapon_upgrade_in_bank() {
            if let Some(equiped_weapon) = self.equipment_in(Slot::Weapon).cloned() {
                if self.action_unequip(unequip_schema::Slot::Weapon).is_ok() {
                    let _ = self.action_deposit(&equiped_weapon.code, 1);
                }
            }
            if self.action_withdraw(&code, 1).is_ok() {
                let _ = self.action_equip(&code, equip_schema::Slot::Weapon);
            }
        }
    }

    fn weapon_upgrade_in_bank(&self) -> Option<String> {
        self.items
            .equipable_at_level(self.data().level, Slot::Weapon)
            .iter()
            .find(|weapon| {
                self.bank
                    .read()
                    .is_ok_and(|b| b.has_item(&weapon.code).is_some())
                    && self.weapon_damage() < weapon.damages()
            })
            .map(|weapon| weapon.code.clone())
    }

    // fn fight_until_unsuccessful(&self, x: i32, y: i32) {
    //     let _ = self.move_to(x, y);

    //     loop {
    //         if let Err(Error::ResponseError(res)) = self.fight() {
    //             if res.status.eq(&StatusCode::from_u16(499).unwrap()) {
    //                 error!("{}: needs to cooldown", self.name);
    //                 self.cool_down(self.remaining_cooldown());
    //             }
    //             if res.status.eq(&StatusCode::from_u16(497).unwrap()) {
    //                 error!("{}: inventory is full", self.name);
    //                 self.move_to_bank();
    //                 self.deposit_all();
    //                 let _ = self.move_to(x, y);
    //             }
    //         }
    //     }
    // }
}

#[derive(Debug, Default, PartialEq, Clone, Deserialize)]
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
