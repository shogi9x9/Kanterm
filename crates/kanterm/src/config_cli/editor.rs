use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;

pub(super) fn open(path: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| anyhow!("VISUAL or EDITOR must be set for `kanterm config edit`"))?;
    let command = split_command(&editor)?;
    let (program, command_args) = command
        .split_first()
        .ok_or_else(|| anyhow!("VISUAL or EDITOR cannot be empty"))?;
    let status = Command::new(program)
        .args(command_args)
        .arg(path)
        .status()
        .with_context(|| format!("starting editor '{program}'"))?;
    if !status.success() {
        return Err(anyhow!("editor exited with status {status}"));
    }
    Ok(())
}

fn split_command(value: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match (quote, character) {
            (_, '\\') if quote != Some('\'') => escaped = true,
            (None, '\'' | '"') => quote = Some(character),
            (Some(active), character) if character == active => quote = None,
            (None, character) if character.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            (_, character) => current.push(character),
        }
    }
    if escaped {
        return Err(anyhow!("editor command ends with an incomplete escape"));
    }
    if quote.is_some() {
        return Err(anyhow!("editor command has an unclosed quote"));
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty() {
        return Err(anyhow!("VISUAL or EDITOR cannot be empty"));
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_supports_flags_quotes_and_escapes() {
        assert_eq!(
            split_command("code --wait --reuse-window").unwrap(),
            ["code", "--wait", "--reuse-window"]
        );
        assert_eq!(
            split_command("'editor app' --flag=some\\ value").unwrap(),
            ["editor app", "--flag=some value"]
        );
        assert!(split_command("code '").is_err());
    }
}
