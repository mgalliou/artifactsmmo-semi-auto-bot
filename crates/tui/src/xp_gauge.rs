use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Gauge, Widget},
};
use sdk::{
    Skill,
    entities::{Character, RawCharacter},
};

use crate::{gauge_ratio, gauge_style};

const XP_STYLE: Style = gauge_style(Color::Green);

pub struct XpGauge {
    char: RawCharacter,
}

impl XpGauge {
    #[must_use]
    pub const fn new(char: RawCharacter) -> Self {
        Self { char }
    }
}

impl Widget for XpGauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let xp = self.char.skill_xp(Skill::Combat);
        let max_xp = self.char.skill_max_xp(Skill::Combat);
        Gauge::default()
            .ratio(gauge_ratio(xp, max_xp))
            .label(format!("{xp} / {max_xp} XP"))
            .gauge_style(XP_STYLE)
            .render(area, buf);
    }
}
