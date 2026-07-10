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
        contrast_fg: Color::Black,
        success: Color::Green,
        warning: Color::Yellow,
        danger: Color::Red,
        priority_high: Color::Red,
        priority_normal: Color::Yellow,
        priority_low: Color::Blue,
        selection_symbol: "",
        column_spacing: 0,
    }
}

pub(super) fn glass_theme() -> Theme {
    Theme {
        help: Color::Rgb(0x94, 0xa3, 0xb8),
        hint: Color::Rgb(0xa5, 0xf3, 0xfc),
        accent: Color::Rgb(0x7d, 0xd3, 0xfc),
        muted: Color::Rgb(0x64, 0x74, 0x8b),
        selected_fg: Color::Rgb(0xba, 0xe6, 0xfd),
        selected_bg: Color::Reset,
        contrast_fg: Color::Rgb(0x0f, 0x17, 0x2a),
        success: Color::Rgb(0x86, 0xef, 0xac),
        warning: Color::Rgb(0xfd, 0xe6, 0x8a),
        danger: Color::Rgb(0xfd, 0xa4, 0xaf),
        priority_high: Color::Rgb(0xfd, 0xa4, 0xaf),
        priority_normal: Color::Rgb(0xfd, 0xe6, 0x8a),
        priority_low: Color::Rgb(0x93, 0xc5, 0xfd),
        selection_symbol: "▎ ",
        column_spacing: 1,
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
        contrast_fg: Color::White,
        success: Color::Green,
        warning: Color::Magenta,
        danger: Color::Red,
        priority_high: Color::Red,
        priority_normal: Color::Magenta,
        priority_low: Color::Blue,
        selection_symbol: "",
        column_spacing: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_theme_keeps_the_terminal_background() {
        let theme = glass_theme();
        assert_eq!(theme.selected_bg, Color::Reset);
        assert_eq!(theme.selection_symbol, "▎ ");
        assert_eq!(theme.column_spacing, 1);
    }
}
