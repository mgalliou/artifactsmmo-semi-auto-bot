use artifactsmmo_openapi::models::ItemSchema;

#[derive(Default)]
pub struct Equipment<'a> {
    pub weapon: Option<&'a ItemSchema>,
    pub shield: Option<&'a ItemSchema>,
    pub helmet: Option<&'a ItemSchema>,
    pub body_armor: Option<&'a ItemSchema>,
    pub leg_armor: Option<&'a ItemSchema>,
    pub boots: Option<&'a ItemSchema>,
    pub ring1: Option<&'a ItemSchema>,
    pub ring2: Option<&'a ItemSchema>,
    pub amulet: Option<&'a ItemSchema>,
    pub artifact1: Option<&'a ItemSchema>,
    pub artifact2: Option<&'a ItemSchema>,
    pub artifact3: Option<&'a ItemSchema>,
    pub consumable1: Option<&'a ItemSchema>,
    pub consumable2: Option<&'a ItemSchema>,
}
