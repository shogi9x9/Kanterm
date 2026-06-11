/// Turn a display name into a URL-ish slug: lowercase, non-alphanumerics become
/// single dashes. Empty input falls back to "board".
pub(crate) fn derive_slug(name: &str) -> String {
    let mut s = String::new();
    let mut pending_dash = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if pending_dash && !s.is_empty() {
                s.push('-');
            }
            pending_dash = false;
            s.extend(c.to_lowercase());
        } else {
            pending_dash = true;
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "board".into()
    } else {
        s
    }
}

/// Derive a key prefix (e.g. "WB" for "Work Board"). ASCII letters only; falls
/// back to "KB" when a name yields nothing usable (e.g. non-Latin scripts).
pub(crate) fn derive_prefix(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let prefix: String = if words.len() <= 1 {
        words
            .first()
            .map(|w| {
                w.chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(3)
                    .collect()
            })
            .unwrap_or_default()
    } else {
        words
            .iter()
            .filter_map(|w| w.chars().find(|c| c.is_ascii_alphanumeric()))
            .take(3)
            .collect()
    };
    let prefix = prefix.to_uppercase();
    if prefix.is_empty() {
        "KB".into()
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_normalizes_spacing_and_symbols() {
        assert_eq!(derive_slug("Work Board"), "work-board");
        assert_eq!(derive_slug("  Work---Board!!  "), "work-board");
        assert_eq!(derive_slug(""), "board");
    }

    #[test]
    fn prefix_uses_ascii_letters_and_falls_back() {
        assert_eq!(derive_prefix("Work Board"), "WB");
        assert_eq!(derive_prefix("backend"), "BAC");
        assert_eq!(derive_prefix("作業"), "KB");
    }
}
