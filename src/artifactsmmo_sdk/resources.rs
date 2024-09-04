use super::{account::Account, api::resources::ResourcesApi};
use artifactsmmo_openapi::models::{resource_schema::Skill, ResourceSchema};

pub struct Resources {
    pub data: Vec<ResourceSchema>,
}

impl Resources {
    pub fn new(account: &Account) -> Resources {
        let api = ResourcesApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
        Resources {
            data: api.all(None, None, None, None).unwrap().clone(),
        }
    }

    pub fn dropping(&self, code: &str) -> Option<Vec<&ResourceSchema>> {
        let monsters = self
            .data
            .iter()
            .filter(|r| r.drops.iter().any(|d| d.code == code))
            .collect::<Vec<_>>();
        match !monsters.is_empty() {
            true => Some(monsters),
            false => None,
        }
    }

    pub fn lowest_providing_exp(
        &self,
        level: i32,
        skill: super::skill::Skill,
    ) -> Option<&ResourceSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|r| Resources::schema_skill_to_skill(r.skill) == skill)
            .filter(|r| r.level >= min && r.level <= level)
            .min_by_key(|r| r.level)
    }

    pub fn highest_providing_exp(
        &self,
        level: i32,
        skill: super::skill::Skill,
    ) -> Option<&ResourceSchema> {
        self.data
            .iter()
            .filter(|r| Resources::schema_skill_to_skill(r.skill) == skill)
            .filter(|r| r.level <= level)
            .max_by_key(|r| r.level)
    }

    pub fn schema_skill_to_skill(skill: Skill) -> super::skill::Skill {
        match skill {
            Skill::Woodcutting => super::skill::Skill::Woodcutting,
            Skill::Mining => super::skill::Skill::Mining,
            Skill::Fishing => super::skill::Skill::Fishing,
        }
    }
}
