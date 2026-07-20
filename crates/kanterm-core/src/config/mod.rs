mod manifest;
mod resolution;

pub use manifest::ConfigManifest;
pub use resolution::{
    global_config_dir, project_config_manifest, resolve_config, resolve_config_with_global,
    validate_config, ConfigResolution, ConfigScope, ResolvedConfigPath,
};

pub const CONFIG_VERSION: u32 = 1;
pub const CONFIG_FILE_NAME: &str = "config.yaml";
pub const CONFIG_TEMPLATE: &str =
    "version: 1\n\n# targets: targets.yaml\n# workflow: workflows/default.yaml\n";
