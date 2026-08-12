use crate::{gauge_ratio, gauge_style};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Gauge, Widget},
};
use sdk::{
    CharacterClient, Skill,
    entities::{Character, RawCharacter},
};
use strum::IntoEnumIterator;

const SKILL_STYLE: Style = gauge_style(Color::White);

#[derive(Default)]
pub struct SkillsWidget {
    char: Option<CharacterClient>,
}

impl SkillsWidget {
    #[must_use]
    pub const fn new(char: Option<CharacterClient>) -> Self {
        Self { char }
    }
}

impl Widget for SkillsWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered().title_top("Skills");
        let inner = block.inner(area);

        block.render(area, buf);
        let snapshot = self.char.as_ref().map(CharacterClient::snapshot);
        let skills = Skill::iter();
        let layout = Layout::vertical(vec![Constraint::Length(1); skills.len()]);
        for (skill, area) in skills.zip(&*layout.split(inner)) {
            xp_bar(snapshot.as_ref(), skill).render(*area, buf);
        }
    }
}

fn xp_bar<'a>(char: Option<&RawCharacter>, skill: Skill) -> Gauge<'a> {
    let level = char.as_ref().map_or(0, |c| c.skill_level(skill));
    let xp = char.as_ref().map_or(0, |c| c.skill_xp(skill));
    let max_xp = char.as_ref().map_or(0, |c| c.skill_max_xp(skill));
    Gauge::default()
        .ratio(gauge_ratio(xp, max_xp))
        .label(format!(
            "{} ({level}): {xp} / {max_xp}",
            skill_to_emoji(skill)
        ))
        .gauge_style(SKILL_STYLE)
}

const fn skill_to_emoji(skill: Skill) -> &'static str {
    match skill {
        Skill::Combat => "󰞇 ",
        Skill::Mining => "󰢷 ",
        Skill::Woodcutting => "󰣈 ",
        Skill::Fishing => "󰈺 ",
        Skill::Weaponcrafting => "󰓥 ",
        Skill::Gearcrafting => " ",
        Skill::Jewelrycrafting => "󰇈 ",
        Skill::Cooking => " ",
        Skill::Alchemy => "󱄯 ",
    }
}
