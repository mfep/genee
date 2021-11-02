//! Utilities to store settings persistently on the disk.
use anyhow::{Context, Result};
use directories_next::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_GRAPH_DAYS: usize = 30;
pub const DEFAULT_PAST_PERIODS: usize = 2;
pub const DEFAULT_MAX_DISPLAYED_COLS: usize = 70;
pub const DEFAULT_LIST_PREVIOUS_DAYS: usize = 0;
const QUALIFIER_ID: &str = "org";
const ORG_ID: &str = "mfep";
const APP_ID: &str = "genee";

/// This struct contains all persistent configuration items.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Path of the default data file.
    pub datafile_path: PathBuf,

    /// How many days to display per iteration.
    pub graph_days: usize,

    /// How many iterations to display.
    pub past_periods: usize,

    /// Maximum number of columns of the content displayed in the termial.
    pub max_displayed_cols: usize,

    /// Specifies the number of days from the diary that should be printed in a tabular format.
    pub list_previous_days: usize,
}

impl std::default::Default for Config {
    fn default() -> Self {
        Config {
            datafile_path: get_default_datafile_path(),
            graph_days: DEFAULT_GRAPH_DAYS,
            past_periods: DEFAULT_PAST_PERIODS,
            max_displayed_cols: DEFAULT_MAX_DISPLAYED_COLS,
            list_previous_days: DEFAULT_LIST_PREVIOUS_DAYS,
        }
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pretty_string_result = toml::to_string_pretty(&self);
        if let Ok(pretty_string) = pretty_string_result {
            write!(f, "{}", pretty_string)
        } else {
            std::fmt::Result::Err(std::fmt::Error)
        }
    }
}

/// Loads the persistent configuration from its default location.
pub fn load_config() -> Result<Config> {
    confy::load_path(get_config_path()).context("Could not load configuration file")
}

/// Saves the persistent configuration to its default location.
pub fn save_config(config: &Config) -> Result<()> {
    Ok(confy::store_path(get_config_path(), config)?)
}

/// Returns the path to the persistent configuration file.
pub fn get_config_path() -> PathBuf {
    let mut config_dir = get_project_dirs().config_dir().to_path_buf();
    config_dir.set_file_name("genee-config");
    config_dir.set_extension("toml");
    config_dir
}

/// Returns the default datafile path.
pub fn get_default_datafile_path() -> PathBuf {
    let mut data_dir = get_project_dirs().data_dir().to_path_buf();
    data_dir.set_file_name("genee-data");
    data_dir.set_extension("csv");
    data_dir
}

fn get_project_dirs() -> ProjectDirs {
    let project_dirs = ProjectDirs::from(QUALIFIER_ID, ORG_ID, APP_ID);
    if project_dirs.is_none() {
        panic!("Cannot open project directory");
    }
    project_dirs.unwrap()
}
