use anyhow::{anyhow, Result};

pub(super) fn trim_comment(line: &str) -> &str {
    line.split_once('#').map(|(left, _)| left).unwrap_or(line)
}

pub(super) fn split_kv(text: &str) -> Option<(&str, &str)> {
    let (key, value) = text.split_once(':')?;
    Some((key.trim(), strip_quotes(value.trim())))
}

fn strip_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

pub(super) fn split_words(value: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in value.chars() {
        match (quote, ch) {
            (Some(active), ch) if ch == active => quote = None,
            (None, '"' | '\'') => quote = Some(ch),
            (None, ch) if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if let Some(active) = quote {
        return Err(anyhow!("unterminated quote {active} in args"));
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(words)
}
