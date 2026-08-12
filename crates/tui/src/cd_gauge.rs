use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Gauge, Widget},
};
use sdk::{CharacterClient, entities::Character};

use crate::gauge_style;

const CD_STYLE: Style = gauge_style(Color::White);

pub struct CdGauge {
    char: CharacterClient,
}

impl CdGauge {
    #[must_use]
    pub const fn new(char: CharacterClient) -> Self {
        Self { char }
    }
}

impl Widget for CdGauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let remaining = self.char.remaining_cooldown();
        let cd = self.char.cooldown();
        let ratio = cd_ratio(remaining, cd);
        let label = format!("{:.1}s", remaining.as_secs_f64());
        Gauge::default()
            .ratio(ratio)
            .label(label)
            .gauge_style(CD_STYLE)
            .render(area, buf);
    }
}

#[must_use]
fn cd_ratio(remaining: Duration, cd: u32) -> f64 {
    if cd < 1 {
        0.0
    } else {
        (remaining.as_secs_f64() / f64::from(cd)).clamp(0.0, 1.0)
    }
}
