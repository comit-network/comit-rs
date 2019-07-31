use super::file::{self, Btsieve, Comit, File, HttpSocket, Network};
use log::LevelFilter;

/// This structs represents the settings as they are used through out the code.
///
/// An optional setting (represented in this struct as an `Option`) has semantic
/// meaning in cnd. Contrary to that, many configuration values are optional in
/// the config file but may be replaced by default values when the `Settings`
/// are created from a given `Config`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub comit: Comit,
    pub network: Network,
    pub http_api: HttpSocket,
    pub btsieve: Btsieve,
    pub web_gui: Option<HttpSocket>,
    pub logging: Logging,
}

impl Settings {
    pub fn from_config_file_and_defaults(config_file: File) -> Self {
        let File {
            comit,
            network,
            http_api,
            btsieve,
            web_gui,
            logging,
        } = config_file;

        Self {
            comit,
            network,
            http_api,
            btsieve,
            web_gui,
            logging: logging
                .map(|logging| Logging {
                    level: logging.level.unwrap_or_else(default_logging_level),
                    structured: logging.structured.unwrap_or(false),
                })
                .unwrap_or_else(|| Logging {
                    level: default_logging_level(),
                    structured: false,
                }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Logging {
    pub level: LevelFilter,
    pub structured: bool,
}

fn default_logging_level() -> LevelFilter {
    LevelFilter::Debug
}

#[cfg(test)]
mod tests {

    use super::*;
    use rand::rngs::OsRng;
    use spectral::prelude::*;

    #[test]
    fn field_structured_defaults_to_false() {
        let config_file = File {
            logging: Some(file::Logging {
                level: None,
                structured: None,
            }),
            ..File::default(OsRng)
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .map(|settings| &settings.logging.structured)
            .is_false()
    }

    #[test]
    fn field_structured_is_correctly_mapped() {
        let config_file = File {
            logging: Some(file::Logging {
                level: None,
                structured: Some(true),
            }),
            ..File::default(OsRng)
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .map(|settings| &settings.logging.structured)
            .is_true()
    }
}
