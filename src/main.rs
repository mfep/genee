use anyhow::{Result, bail};
use clap::Parser;
use genee::datafile;
use std::path::{Path, PathBuf};

mod configuration;
mod ui;

#[derive(Parser, Clone)]
#[command(version, about)]
struct CliOptions {
    /// Path to the diary file.
    /// When not provided, its value is loaded from persistent configuration file.
    #[arg(short, long)]
    datafile: Option<PathBuf>,

    /// Specifies the number of displayed periods when graphing the diary data.
    /// When not provided, its value is loaded from persistent configuration file.
    #[arg(short, long)]
    past_periods: Option<usize>,

    /// Specifies the number of most frequent daily habit compositions over the specified period.
    #[arg(short = 'f', long)]
    list_most_frequent_days: Option<usize>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Parser, Clone)]
enum Command {
    /// Prints the persistent configuration.
    ListConfig,

    /// Provide a comma separated list of habit categories. A new diary file is created at the specified
    /// --datafile path.
    New { category_list: String },

    /// Saves the specified options to persistent configuration.
    SaveConfig,

    /// Adds or unhides a category.
    AddCategory { name: String },

    /// Hides a category.
    HideCategory { name: String },
}

fn main() -> Result<()> {
    let opt = handle_config()?;
    let datafile_path = opt.datafile.as_ref().unwrap();
    match opt.command {
        Some(Command::ListConfig) => {
            let persistent_config = configuration::load_config()?;
            println!(
                "Listing persistent configuration loaded from '{}'\n{}",
                configuration::get_config_path().to_string_lossy(),
                &persistent_config
            );
        }
        Some(Command::New { ref category_list }) => {
            create_new(datafile_path, category_list)?;
        }
        Some(Command::SaveConfig) => {
            configuration::save_config_opt(&opt)?;
        }
        Some(Command::AddCategory { ref name }) => {
            add_category(datafile_path, name)?;
        }
        Some(Command::HideCategory { ref name }) => {
            hide_category(datafile_path, name)?;
        }
        None => {
            ui::run_app(&opt)?;
        }
    }

    Ok(())
}

fn handle_config() -> Result<CliOptions> {
    let opt = CliOptions::parse();
    let persistent_config = configuration::load_config()?;
    let opt = merge_cli_and_persistent_options(opt, &persistent_config);
    Ok(opt)
}

fn merge_cli_and_persistent_options(
    options_from_cli: CliOptions,
    persistent_config: &configuration::Config,
) -> CliOptions {
    CliOptions {
        datafile: options_from_cli
            .datafile
            .or_else(|| Some(persistent_config.datafile_path.clone())),
        past_periods: options_from_cli
            .past_periods
            .or(Some(persistent_config.past_periods)),
        list_most_frequent_days: options_from_cli
            .list_most_frequent_days
            .or(Some(persistent_config.list_most_frequent_days)),
        ..options_from_cli
    }
}

fn create_new(path: &Path, headers_string: &str) -> Result<()> {
    let mut headers_vector = vec![];
    for title in headers_string.split(',') {
        if title.is_empty() {
            bail!(format!("Invalid header specification: {}", headers_string));
        }
        headers_vector.push(String::from(title));
    }
    datafile::create_new_datafile(path, &headers_vector)?;
    println!("New datafile successfully created at {}", path.display());
    Ok(())
}

fn add_category(datafile_path: &Path, name: &str) -> Result<()> {
    let datafile = datafile::open_datafile(datafile_path)?;
    match datafile.add_category(name)? {
        datafile::AddCategoryResult::AddedNew => {
            println!("Added new category \"{}\"", name);
        }
        datafile::AddCategoryResult::AlreadyPresent => {
            bail!(
                "Category \"{}\" was already present and shown in the datafile",
                name
            );
        }
        datafile::AddCategoryResult::Unhide => {
            println!("Previously hidden category is showed again: \"{}\"", name)
        }
    }
    Ok(())
}

fn hide_category(datafile_path: &Path, name: &str) -> Result<()> {
    let datafile = datafile::open_datafile(datafile_path)?;
    match datafile.hide_category(name)? {
        datafile::HideCategoryResult::AlreadyHidden => {
            bail!("Category \"{}\" was already hidden", name)
        }
        datafile::HideCategoryResult::NonExistingCategory => {
            bail!("Category \"{}\" does not exist", name)
        }
        datafile::HideCategoryResult::Hidden => {
            println!("Category \"{}\" is hidden", name);
        }
    }
    Ok(())
}
