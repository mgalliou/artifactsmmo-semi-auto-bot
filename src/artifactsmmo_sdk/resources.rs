use super::{account::Account, api::resources::ResourcesApi};

pub struct Resources {
    api: ResourcesApi,
}

impl Resources {
    pub fn new(account: &Account) -> Resources {
        Resources {
            api: ResourcesApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    pub fn dropping(&self, code: &str) -> Option<Vec<String>> {
        let mut codes: Vec<String> = vec![];

        if let Ok(resources) = self.api.all(None, None, None, Some(code), None, None) {
            for r in resources.data {
                codes.push(r.code)
            }
            return Some(codes);
        }
        None
    }
}
