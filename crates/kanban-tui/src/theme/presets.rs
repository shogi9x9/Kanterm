use ratatui::style::Color;

use super::Theme;

pub(super) fn dark_theme() -> Theme {
    Theme {
        help: Color::Gray,
        hint: Color::LightCyan,
        accent: Color::Cyan,
        muted: Color::DarkGray,
        selected_fg: Color::Black,
        selected_bg: Color::Cyan,
        success: Color::Green,
        warning: Color::Yellow,
        danger: Color::Red,
        priority_high: Color::Red,
        priority_normal: Color::Yellow,
        priority_low: Color::Blue,
    }
}

pub(super) fn light_theme() -> Theme {
    Theme {
        help: Color::DarkGray,
        hint: Color::Blue,
        accent: Color::Blue,
        muted: Color::Gray,
        selected_fg: Color::White,
        selected_bg: Color::Blue,
        success: Color::Green,
        warning: Color::Magenta,
        danger: Color::Red,
        priority_high: Color::Red,
        priority_normal: Color::Magenta,
        priority_low: Color::Blue,
    }
}
