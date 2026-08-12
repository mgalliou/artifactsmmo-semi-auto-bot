use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Gauge, Widget},
};
use sdk::entities::{Character, RawCharacter};

use crate::{gauge_ratio, gauge_style};

const HEALTH_STYLE: Style = gauge_style(Color::Red);

pub struct HealthGauge {
    char: RawCharacter,
}

impl HealthGauge {
    #[must_use]
    pub const fn new(char: RawCharacter) -> Self {
        Self { char }
    }
}

impl Widget for HealthGauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hp = self.char.hp();
        let max_hp = self.char.max_hp();
        Gauge::default()
            .ratio(gauge_ratio(hp, max_hp))
            .label(format!("{hp} / {max_hp} HP"))
            .gauge_style(HEALTH_STYLE)
            .render(area, buf);
    }
}
