use crate::{settings::CndSettings, std_ext::path::PrintablePathBuf};
use config::ConfigError;
use rand::Rng;
use std::path::{Path, PathBuf};

pub fn default_config_path(parent: &Path) -> PathBuf {
    let user_path_components: PathBuf = [".config", "comit", "cnd.toml"].iter().collect();

    parent.join(user_path_components)
}

pub fn load_settings<R: Rng>(
    config_file_path_override: Option<&PathBuf>,
    home_dir: Option<&Path>,
    rand: R,
) -> Result<CndSettings, ConfigError> {
    match (config_file_path_override, home_dir) {
        (None, Some(home_dir)) => {
            let default_config_file_path = default_config_path(home_dir);
            println!(
                "No config file path override was specified on the command line, defaulting to {}",
                PrintablePathBuf(&default_config_file_path)
            );

            if default_config_file_path.exists() {
                CndSettings::read(default_config_file_path)
            } else {
                println!(
                    "Creating default config file at {} because it does not exist yet",
                    PrintablePathBuf(&default_config_file_path)
                );
                CndSettings::default(rand).write_to(default_config_file_path)
            }
        }
        (Some(config_file_path_override), _) => {
            println!(
                "Reading config file from {}",
                PrintablePathBuf(&config_file_path_override)
            );
            CndSettings::read(config_file_path_override)
        }
        (None, None) => {
            eprintln!("Failed to determine home directory and hence could not infer default config file location. You can directly pass a config file through `--config`.");
            Err(ConfigError::Message(
                "Failed to determine home directory".to_owned(),
            ))
        }
    }
}
