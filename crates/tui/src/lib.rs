pub mod app;
pub mod bank;
pub mod cd_gauge;
pub mod chars_info;
pub mod health_gauge;
pub mod inventory;
pub mod log;
pub mod map;
pub mod orderboard;
pub mod skills_widget;
pub mod xp_gauge;

pub use app::App;
pub use log::init_logger;

use ratatui::style::{Color, Style};

const GAUGE_BG: Color = Color::Black;

#[must_use]
const fn gauge_style(fg: Color) -> Style {
    Style::new().bg(GAUGE_BG).fg(fg)
}

#[must_use]
pub fn gauge_ratio(value: i32, max: i32) -> f64 {
    if max <= 0 {
        1.0
    } else {
        (f64::from(value) / f64::from(max)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::gauge_ratio;

    fn assert_ratio(value: i32, max: i32, expected: f64) {
        assert!((gauge_ratio(value, max) - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn gauge_ratio_handles_non_positive_maxima() {
        assert_ratio(0, 0, 1.0);
        assert_ratio(10, -1, 1.0);
    }

    #[test]
    fn gauge_ratio_clamps_values() {
        assert_ratio(-1, 10, 0.0);
        assert_ratio(5, 10, 0.5);
        assert_ratio(11, 10, 1.0);
    }
}
