mod args;
mod commands;
mod process;
mod settings;

use anyhow::Result;

pub(crate) use args::{parse, usage, HookCommand};

pub(crate) fn run(command: HookCommand) -> Result<()> {
    match command {
        HookCommand::Install(install) => {
            settings::apply_settings(&install)?;
            println!(
                "hooks installed: mode={} settings={}",
                args::mode_name(install.mode),
                install.settings.display()
            );
        }
        HookCommand::Uninstall { settings } => {
            settings::apply_off(&settings)?;
            println!("hooks uninstalled: settings={}", settings.display());
        }
        HookCommand::Status { settings } => settings::print_status(&settings)?,
        HookCommand::StopWatcher { for_agent, run_dir } => {
            process::stop_watcher(&run_dir, &for_agent)?;
        }
        HookCommand::Help => println!("{}", usage()),
    }
    Ok(())
}
