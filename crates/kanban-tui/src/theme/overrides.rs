use anyhow::{Context, Result};
use serde::Deserialize;

use super::parser::parse_color;
use super::Theme;

#[derive(Default, Deserialize)]
pub(super) struct ThemeOverride {
    help: Option<String>,
    hint: Option<String>,
    accent: Option<String>,
    muted: Option<String>,
    selected_fg: Option<String>,
    selected_bg: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    danger: Option<String>,
    priority_high: Option<String>,
    priority_normal: Option<String>,
    priority_low: Option<String>,
}

pub(super) fn apply_theme_override(t: &mut Theme, o: ThemeOverride) -> Result<()> {
    macro_rules! set_color {
        ($field:ident) => {
            if let Some(value) = o.$field {
                t.$field = parse_color(&value)
                    .with_context(|| format!("theme color '{}'", stringify!($field)))?;
            }
        };
    }
    set_color!(help);
    set_color!(hint);
    set_color!(accent);
    set_color!(muted);
    set_color!(selected_fg);
    set_color!(selected_bg);
    set_color!(success);
    set_color!(warning);
    set_color!(danger);
    set_color!(priority_high);
    set_color!(priority_normal);
    set_color!(priority_low);
    Ok(())
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crate::theme::presets::dark_theme;

    #[test]
    fn override_updates_only_named_fields() {
        let mut t = dark_theme();
        apply_theme_override(
            &mut t,
            ThemeOverride {
                help: Some("white".into()),
                priority_high: Some("#ff2200".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(t.help, Color::White);
        assert_eq!(t.priority_high, Color::Rgb(0xff, 0x22, 0x00));
        assert_eq!(t.priority_low, dark_theme().priority_low);
    }
}
