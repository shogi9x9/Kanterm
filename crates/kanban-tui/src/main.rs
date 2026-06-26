//! kanterm: a ratatui board for a local terminal.
//!
//! Synchronous by design (no tokio): rusqlite is fast and local, and the usage
//! model is "open, do one thing, close". All persistence goes through
//! `kanban_core::Store`.

mod app;
mod editor;
mod layout;
mod mode;
mod theme;

use anyhow::{anyhow, Context, Result};
use app::App;
use kanban_core::{Store, SCHEMA_VERSION};
use std::path::Path;
use theme::init_theme;

fn main() -> Result<()> {
    let path = match std::env::var_os("KANBAN_DB") {
        Some(p) => std::path::PathBuf::from(p),
        None => Store::default_db_path()?,
    };

    // Headless export mode: `kanterm --export json|md`. No TUI.
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--backup-db") {
        let destination = args
            .get(pos + 1)
            .ok_or_else(|| anyhow!("--backup-db requires a destination path"))?;
        let store = Store::open(&path)?;
        store.backup_to(destination)?;
        println!("backup written: {} -> {destination}", path.display());
        return Ok(());
    }
    if let Some(pos) = args.iter().position(|a| a == "--restore-db") {
        let source = args
            .get(pos + 1)
            .ok_or_else(|| anyhow!("--restore-db requires a source database path"))?;
        let force = args.iter().any(|a| a == "--force");
        restore_db(source, &path, force)?;
        println!("restored {} -> {}", source, path.display());
        return Ok(());
    }
    if let Some(pos) = args.iter().position(|a| a == "--export") {
        let fmt = args.get(pos + 1).map(String::as_str).unwrap_or("json");
        let mut store = Store::open(&path)?;
        let board = store.ensure_default_board()?;
        let out = match fmt {
            "md" | "markdown" => store.export_markdown(&board.id)?,
            _ => store.export_json(&board.id)?,
        };
        println!("{out}");
        return Ok(());
    }

    let mut store = Store::open(&path)?;
    let board = store.ensure_default_board()?;
    init_theme()?;
    let mut app = App::new(store, board)?;

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}

fn restore_db(source: impl AsRef<Path>, destination: impl AsRef<Path>, force: bool) -> Result<()> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    let version = Store::database_schema_version(source)?;
    if version > SCHEMA_VERSION {
        return Err(anyhow!(
            "backup schema version {version} is newer than this build supports ({SCHEMA_VERSION}); update kanban before restoring"
        ));
    }
    if destination.exists() && !force {
        return Err(anyhow!(
            "destination {} already exists; pass --force to replace it",
            destination.display()
        ));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating data dir {}", parent.display()))?;
    }
    if force {
        remove_if_exists(destination)?;
        remove_if_exists(wal_path(destination))?;
        remove_if_exists(shm_path(destination))?;
    }
    std::fs::copy(source, destination).with_context(|| {
        format!(
            "copying backup {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn remove_if_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}

fn wal_path(path: &Path) -> std::path::PathBuf {
    path.with_file_name(format!(
        "{}-wal",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}

fn shm_path(path: &Path) -> std::path::PathBuf {
    path.with_file_name(format!(
        "{}-shm",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}
