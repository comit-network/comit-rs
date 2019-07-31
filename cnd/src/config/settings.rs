use super::file::{Btsieve, Comit, File, HttpSocket, Network};
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
    pub log_levels: LogLevels,
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
            log_levels: logging
                .map(|log_levels| LogLevels {
                    cnd: log_levels.level.unwrap_or_else(default_cnd_level_filter),
                })
                .unwrap_or_else(|| LogLevels {
                    cnd: default_cnd_level_filter(),
                }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogLevels {
    pub cnd: LevelFilter,
}

fn default_cnd_level_filter() -> LevelFilter {
    LevelFilter::Debug
}
