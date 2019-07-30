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
    if let Some(config_file) = config_path_override {
        println!("Using config file {}", PrintablePath(config_file));
        return CndSettings::read(config_file);
    }

    let default_config_path = home_dir.map(default_config_path).ok_or_else(|| {
        eprintln!("Failed to determine home directory and hence could not infer default config file location. You can specify a config file with `--config`.");
        ConfigError::Message(
            "Failed to determine home directory".to_owned(),
        )
    })?;

    if default_config_path.exists() {
        println!("Using config file {}", PrintablePath(&default_config_path));
        CndSettings::read(default_config_path)
    } else {
        println!(
            "Creating config file at {} because it does not exist yet",
            PrintablePath(&default_config_path)
        );
        CndSettings::default(rand).write_to(default_config_path)
    }
}
