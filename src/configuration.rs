//! Utilities to store settings persistently on the disk.
use anyhow::Result;
use directories_next::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use crate::CliOptions;

pub const DEFAULT_PAST_PERIODS: usize = 2;
pub const DEFAULT_LIST_MOST_FREQUENT_DAYS: usize = 5;
const QUALIFIER_ID: &str = "xyz";
const ORG_ID: &str = "safeworlds";
const APP_ID: &str = "genee";

/// This struct contains all persistent configuration items.
#[derive(Serialize)]
pub struct Config {
    /// Path of the default data file.
    pub datafile_path: PathBuf,

    /// How many iterations to display.
    pub past_periods: usize,

    /// Specifies the number of most frequent daily habit compositions to print
    pub list_most_frequent_days: usize,
}

#[derive(Serialize, Deserialize, Default)]
struct SerializedConfig {
    datafile_path: Option<PathBuf>,
    past_periods: Option<usize>,
    list_most_frequent_days: Option<usize>,
}

impl SerializedConfig {
    fn into_config(self) -> Config {
        Config {
            datafile_path: self.datafile_path.unwrap_or(get_default_datafile_path()),
            past_periods: self.past_periods.unwrap_or(DEFAULT_PAST_PERIODS),
            list_most_frequent_days: self
                .list_most_frequent_days
                .unwrap_or(DEFAULT_LIST_MOST_FREQUENT_DAYS),
        }
    }

    fn from_config(config: &Config) -> Self {
        SerializedConfig {
            datafile_path: Some(config.datafile_path.clone()),
            past_periods: Some(config.past_periods),
            list_most_frequent_days: Some(config.list_most_frequent_days),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        SerializedConfig::default().into_config()
    }
}

impl Display for Config {
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
    let path = get_config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let mut config_content = String::default();
    File::open(path)?.read_to_string(&mut config_content)?;

    let deserialized_config: SerializedConfig = toml::from_str(&config_content)?;
    Ok(deserialized_config.into_config())
}

pub fn save_config_opt(opt: &CliOptions) -> Result<()> {
    let provided_datafile_path = opt
        .datafile
        .clone()
        .unwrap_or_else(get_default_datafile_path);
    let full_datafile_path = std::fs::canonicalize(provided_datafile_path.clone());
    if full_datafile_path.is_err() {
        println!("Cannot canonicalize provided datafile path, saving the uncanonicalized path to configuration");
    }
    let updated_config = Config {
        datafile_path: full_datafile_path.unwrap_or(provided_datafile_path),
        past_periods: opt.past_periods.unwrap_or(DEFAULT_PAST_PERIODS),
        list_most_frequent_days: opt
            .list_most_frequent_days
            .unwrap_or(DEFAULT_LIST_MOST_FREQUENT_DAYS),
    };
    save_config(&updated_config)?;
    Ok(())
}

/// Saves the persistent configuration to its default location.
pub fn save_config(config: &Config) -> Result<()> {
    let serialized_config = SerializedConfig::from_config(config);
    let serialized_config = toml::to_string(&serialized_config)?;

    let path = get_config_path();
    fs::create_dir_all(path.parent().unwrap())?;
    let mut file = File::create(path)?;
    write!(file, "{}", serialized_config)?;
    Ok(())
}

/// Returns the path to the persistent configuration file.
pub fn get_config_path() -> PathBuf {
    get_project_dirs().config_dir().join("config.toml")
}

/// Returns the default datafile path.
pub fn get_default_datafile_path() -> PathBuf {
    get_project_dirs().data_dir().join("genee.db")
}

fn get_project_dirs() -> ProjectDirs {
    let project_dirs = ProjectDirs::from(QUALIFIER_ID, ORG_ID, APP_ID);
    if project_dirs.is_none() {
        panic!("Cannot open project directory");
    }
    project_dirs.unwrap()
}
