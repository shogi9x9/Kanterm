mod overrides;
mod parser;
mod presets;

use anyhow::{anyhow, Context, Result};
use kanterm_core::{priority_badge, PRIORITY_HIGH, PRIORITY_LOW};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use std::sync::OnceLock;

use overrides::{apply_theme_override, ThemeOverride};
pub(crate) use parser::hex_to_color;
use presets::{dark_theme, light_theme};

static THEME: OnceLock<Theme> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) struct Theme {
    pub help: Color,
    pub hint: Color,
    pub accent: Color,
    pub muted: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub priority_high: Color,
    pub priority_normal: Color,
    pub priority_low: Color,
}

pub(crate) fn theme() -> &'static Theme {
    THEME.get_or_init(dark_theme)
}

pub(crate) fn init_theme() -> Result<()> {
    let name = std::env::var("KANBAN_THEME").unwrap_or_else(|_| "dark".into());
    let mut t = match name.as_str() {
        "dark" => dark_theme(),
        "light" => light_theme(),
        other => return Err(anyhow!("unknown KANBAN_THEME '{other}'; use dark or light")),
    };
    if let Some(path) = std::env::var_os("KANBAN_THEME_FILE") {
        let text = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading theme file {}",
                std::path::Path::new(&path).display()
            )
        })?;
        let overrides: ThemeOverride = serde_json::from_str(&text).with_context(|| {
            format!(
                "parsing theme file {}",
                std::path::Path::new(&path).display()
            )
        })?;
        apply_theme_override(&mut t, overrides)?;
    }
    let _ = THEME.set(t);
    Ok(())
}

fn priority_style(priority: i64) -> Style {
    match priority {
        PRIORITY_HIGH => Style::default()
            .fg(theme().priority_high)
            .add_modifier(Modifier::BOLD),
        PRIORITY_LOW => Style::default().fg(theme().priority_low),
        _ => Style::default().fg(theme().priority_normal),
    }
}

pub(crate) fn priority_span(priority: i64) -> Span<'static> {
    Span::styled(priority_badge(priority), priority_style(priority))
}
