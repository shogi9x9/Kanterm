/// Escape LIKE wildcards so user text matches literally (pairs with ESCAPE '\').
pub(crate) fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub(crate) fn trimmed_optional(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_escape_treats_wildcards_literally() {
        assert_eq!(like_escape(r"a%b_c\d"), r"a\%b\_c\\d");
    }

    #[test]
    fn trimmed_optional_clears_blank_values() {
        assert_eq!(trimmed_optional(" next "), Some("next"));
        assert_eq!(trimmed_optional("   "), None);
    }
}
