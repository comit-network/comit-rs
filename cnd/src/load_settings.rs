use crate::{settings::CndSettings, std_ext::path::PrintablePath};
use config::ConfigError;
use rand::Rng;
use std::path::{Path, PathBuf};

pub fn default_config_path(parent: &Path) -> PathBuf {
    let user_path_components: PathBuf = [".config", "comit", "cnd.toml"].iter().collect();

    parent.join(user_path_components)
}

pub fn load_settings<R: Rng>(
    config_path_override: Option<&PathBuf>,
    home_dir: Option<&Path>,
    rand: R,
) -> Result<CndSettings, ConfigError> {
    let default_config_path = home_dir.map(default_config_path);

    match (config_path_override, default_config_path) {
        (None, Some(ref default_config_path)) if default_config_path.exists() => {
            println!("Using config file {}", PrintablePath(&default_config_path));
            CndSettings::read(default_config_path)
        }
        (Some(config_path_override), _) => {
            println!("Using config file {}", PrintablePath(&config_path_override));
            CndSettings::read(config_path_override)
        }
        (None, Some(default_config_path)) => {
            println!(
                "Creating config file at {} because it does not exist yet",
                PrintablePath(&default_config_path)
            );
            CndSettings::default(rand).write_to(default_config_path)
        }
        (None, None) => {
            eprintln!("Failed to determine home directory and hence could not infer default config file location. You can specify a config file with `--config`.");
            Err(ConfigError::Message(
                "Failed to determine home directory".to_owned(),
            ))
        }
    }
}
