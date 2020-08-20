use crate::{cli::Options, config, config::Settings};
use anyhow::Context;
use std::{
    env,
    path::{Path, PathBuf},
};

#[allow(clippy::print_stdout)] // We cannot use `log` before we have the config file
pub fn read_config(options: &Options) -> anyhow::Result<config::File> {
    // if the user specifies a config path, use it
    if let Some(path) = &options.config_file {
        eprintln!("Using config file {}", path.display());

        return config::File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    // try to load default config
    let default_path = default_config_path()?;

    if !default_path.exists() {
        return Ok(config::File::default());
    }

    eprintln!(
        "Using config file at default path: {}",
        default_path.display()
    );

    config::File::read(&default_path)
        .with_context(|| format!("failed to read config file {}", default_path.display()))
}

#[allow(clippy::print_stdout)] // Don't use the logger so its easier to cut'n'paste
pub fn dump_config(settings: Settings) -> anyhow::Result<()> {
    let file = config::File::from(settings);
    let serialized = toml::to_string(&file)?;
    println!("{}", serialized);
    Ok(())
}

fn default_config_path() -> anyhow::Result<PathBuf> {
    config_dir()
        .map(|dir| Path::join(&dir, "cnd.toml"))
        .context("Could not generate default configuration path")
}

// Linux: /home/<user>/.config/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\config\
// OSX: /Users/<user>/Library/Application Support/comit/
fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "comit")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}

// Linux: /home/<user>/.local/share/comit/
// Windows: C:\Users\<user>\AppData\Roaming\comit\
// OSX: /Users/<user>/Library/Application Support/comit/
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "comit")
        .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
}

/// Returns `/Users/[username]/Library/Application Support/Lnd/` for macos.
/// Returns `%LOCALAPPDATA%/Lnd for windows.
/// Returns `~/.lnd` if $HOME exists for linux.
pub fn lnd_default_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        directories::ProjectDirs::from("", "", "Lnd")
            .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
    } else if cfg!(target_os = "linux") {
        directories::UserDirs::new().map(|d| d.home_dir().to_path_buf().join(".lnd"))
    } else {
        None
    }
}

/// Returns the directory used by lnd.
pub fn lnd_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("LND_DIR") {
        return Some(PathBuf::from(dir));
    }
    lnd_default_dir()
}
