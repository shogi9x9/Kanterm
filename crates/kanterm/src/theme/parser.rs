use anyhow::{anyhow, Result};
use ratatui::style::Color;

pub(super) fn parse_color(s: &str) -> Result<Color> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    let color = match lower.as_str() {
        "reset" | "default" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" | "dark-grey" | "darkgrey" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => {
            if let Some(hex) = s.strip_prefix('#') {
                return parse_hex_color(hex);
            }
            return Err(anyhow!("unknown color '{s}'"));
        }
    };
    Ok(color)
}

fn parse_hex_color(hex: &str) -> Result<Color> {
    if hex.len() != 6 {
        return Err(anyhow!("hex colors must be #RRGGBB"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok(Color::Rgb(r, g, b))
}

pub(crate) fn hex_to_color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    Color::Gray
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_parser_accepts_names_and_hex() {
        assert_eq!(parse_color("light_cyan").unwrap(), Color::LightCyan);
        assert_eq!(parse_color("default").unwrap(), Color::Reset);
        assert_eq!(
            parse_color("#123abc").unwrap(),
            Color::Rgb(0x12, 0x3a, 0xbc)
        );
        assert!(parse_color("not-a-color").is_err());
        assert!(parse_color("#123").is_err());
    }

    #[test]
    fn hex_to_color_falls_back_to_gray() {
        assert_eq!(hex_to_color("#zzzzzz"), Color::Gray);
        assert_eq!(hex_to_color("#102030"), Color::Rgb(0x10, 0x20, 0x30));
    }
}
