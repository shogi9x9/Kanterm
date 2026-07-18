use anyhow::{anyhow, Context, Result};
use kanterm_core::{
    resolve_config_with_global, validate_config, ConfigManifest, ResolvedConfigPath,
    CONFIG_FILE_NAME, CONFIG_TEMPLATE,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::editor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Global,
    Project,
}

pub(super) fn run(args: &[String], cwd: &Path, global_dir: &Path) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("path") => command_path(&args[1..], cwd, global_dir),
        Some("show") => command_show(&args[1..], cwd, global_dir),
        Some("init") => command_init(&args[1..], cwd, global_dir),
        Some("edit") => command_edit(&args[1..], cwd, global_dir),
        Some("validate") => command_validate(&args[1..], cwd, global_dir),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(anyhow!(
            "unknown config command '{other}'; run `kanterm config --help`"
        )),
    }
}

fn command_path(args: &[String], cwd: &Path, global_dir: &Path) -> Result<()> {
    let scope = optional_scope(args)?;
    let global_manifest = global_dir.join(CONFIG_FILE_NAME);
    let project_manifest = kanterm_core::project_config_manifest(cwd);
    match scope {
        Some(Scope::Global) => println!("{}", global_manifest.display()),
        Some(Scope::Project) => println!("{}", project_manifest.display()),
        None => {
            println!("global:  {}", global_manifest.display());
            println!("project: {}", project_manifest.display());
        }
    }
    Ok(())
}

fn command_show(args: &[String], cwd: &Path, global_dir: &Path) -> Result<()> {
    let resolved = match args {
        [] => false,
        [flag] if flag == "--resolved" => true,
        _ => return Err(anyhow!("usage: kanterm config show [--resolved]")),
    };
    if resolved {
        let resolution = resolve_config_with_global(cwd, global_dir, None, None)?;
        println!("version: 1");
        print_resolved_path("targets", resolution.targets.as_ref());
        print_resolved_path("workflow", resolution.workflow.as_ref());
        return Ok(());
    }
    print_manifest("global", &global_dir.join(CONFIG_FILE_NAME))?;
    print_manifest("project", &kanterm_core::project_config_manifest(cwd))?;
    Ok(())
}

fn command_init(args: &[String], cwd: &Path, global_dir: &Path) -> Result<()> {
    let scope = required_scope(args, "init")?;
    let path = manifest_for_scope(scope, cwd, global_dir);
    init_manifest(&path)?;
    println!("created {}", path.display());
    Ok(())
}

fn command_edit(args: &[String], cwd: &Path, global_dir: &Path) -> Result<()> {
    let scope = required_scope(args, "edit")?;
    let path = manifest_for_scope(scope, cwd, global_dir);
    if !path.is_file() {
        return Err(anyhow!(
            "config does not exist: {}; run `kanterm config init {}` first",
            path.display(),
            scope.flag()
        ));
    }
    editor::open(&path)?;
    validate_manifest_references(&path)?;
    println!("validated {}", path.display());
    Ok(())
}

fn command_validate(args: &[String], cwd: &Path, global_dir: &Path) -> Result<()> {
    if !args.is_empty() {
        return Err(anyhow!("usage: kanterm config validate"));
    }
    let resolution = resolve_config_with_global(cwd, global_dir, None, None)?;
    let manifests = [
        resolution.global_manifest.as_path(),
        resolution.project_manifest.as_path(),
    ];
    if !manifests.iter().any(|path| path.is_file()) {
        return Err(anyhow!(
            "no config manifest found; run `kanterm config init --project` or `kanterm config init --global`"
        ));
    }
    for path in manifests.into_iter().filter(|path| path.is_file()) {
        validate_manifest_references(path)?;
    }
    validate_config(&resolution)?;
    println!("config valid");
    print_resolved_path("targets", resolution.targets.as_ref());
    print_resolved_path("workflow", resolution.workflow.as_ref());
    Ok(())
}

fn optional_scope(args: &[String]) -> Result<Option<Scope>> {
    match args {
        [] => Ok(None),
        [flag] if flag == "--global" => Ok(Some(Scope::Global)),
        [flag] if flag == "--project" => Ok(Some(Scope::Project)),
        _ => Err(anyhow!("expected either --global or --project")),
    }
}

fn required_scope(args: &[String], command: &str) -> Result<Scope> {
    optional_scope(args)?
        .ok_or_else(|| anyhow!("usage: kanterm config {command} --global|--project"))
}

impl Scope {
    const fn flag(self) -> &'static str {
        match self {
            Self::Global => "--global",
            Self::Project => "--project",
        }
    }
}

fn manifest_for_scope(scope: Scope, cwd: &Path, global_dir: &Path) -> PathBuf {
    match scope {
        Scope::Global => global_dir.join(CONFIG_FILE_NAME),
        Scope::Project => kanterm_core::project_config_manifest(cwd),
    }
}

pub(super) fn init_manifest(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "creating config manifest {} (already exists?)",
                path.display()
            )
        })?;
    file.write_all(CONFIG_TEMPLATE.as_bytes())
        .with_context(|| format!("writing config manifest {}", path.display()))
}

fn validate_manifest_references(path: &Path) -> Result<()> {
    let manifest = ConfigManifest::load(path)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for (kind, value) in [
        ("targets", manifest.targets),
        ("workflow", manifest.workflow),
    ] {
        if let Some(value) = value {
            let resolved = if value.is_absolute() {
                value
            } else {
                base.join(value)
            };
            if !resolved.is_file() {
                return Err(anyhow!(
                    "{kind} file does not exist: {} (declared in {})",
                    resolved.display(),
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn print_manifest(label: &str, path: &Path) -> Result<()> {
    println!("# {label}: {}", path.display());
    if path.is_file() {
        let source = fs::read_to_string(path)
            .with_context(|| format!("reading config manifest {}", path.display()))?;
        print!("{source}");
        if !source.ends_with('\n') {
            println!();
        }
    } else {
        println!("# not initialized");
    }
    Ok(())
}

fn print_resolved_path(label: &str, value: Option<&ResolvedConfigPath>) {
    match value {
        Some(value) => println!(
            "{label}: {} (source: {})",
            value.path.display(),
            value.source.as_str()
        ),
        None => println!("{label}: <not configured>"),
    }
}

fn print_help() {
    println!(
        "kanterm config commands:\n  path [--global|--project]\n  show [--resolved]\n  init --global|--project\n  edit --global|--project\n  validate"
    );
}
