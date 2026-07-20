use super::commands::{init_manifest, run};
use kanterm_core::{ConfigManifest, CONFIG_FILE_NAME, CONFIG_TEMPLATE};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("kanterm-config-cli-{name}-{unique}"))
}

#[test]
fn init_never_overwrites_an_existing_manifest() {
    let root = temp_dir("init");
    let path = root.join(CONFIG_FILE_NAME);
    init_manifest(&path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), CONFIG_TEMPLATE);
    assert!(init_manifest(&path).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), CONFIG_TEMPLATE);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn context_commands_initialize_and_validate_project_config() {
    let root = temp_dir("commands");
    let repo = root.join("repo");
    let global = root.join("global");
    fs::create_dir_all(repo.join(".git")).unwrap();
    run(&["init".into(), "--project".into()], &repo, &global).unwrap();
    run(&["validate".into()], &repo, &global).unwrap();
    let path = repo.join(".kanterm").join(CONFIG_FILE_NAME);
    assert_eq!(ConfigManifest::load(&path).unwrap().version, 1);
    let _ = fs::remove_dir_all(root);
}
