use anyhow::{bail, Context, Result};
use chrono::Duration;
use chrono::Local;
use chrono::NaiveDate;
use dialoguer::MultiSelect;
use genee::configuration;
use genee::datafile;
use genee::datafile::DiaryDataConnection;
use genee::graphing;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about)]
struct CliOptions {
    /// Path to the diary file.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long, parse(from_os_str))]
    datafile: Option<PathBuf>,

    /// How many days each period should contain.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long)]
    graph_days: Option<usize>,

    /// Specifies the number of displayed periods when graphing the diary data.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long)]
    past_periods: Option<usize>,

    /// Specifies the number of days from the diary that should be printed in a tabular format.
    #[structopt(short, long)]
    list_previous_days: Option<usize>,

    /// Specifies the maximum allowed width of the terminal output.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(long)]
    max_displayed_cols: Option<usize>,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    /// If set, habit information for all the missing days is queried between --from-date
    /// and yesterday. If --from-date is not set, all the missing days are queried between the
    /// first entry in the diary and yesterday.
    /// If there is no entry in the diary, only yesterday is queried.
    Fill {
        /// Start the querying from this date. Must be in format YYYY-MM-DD.
        #[structopt(long)]
        from_date: Option<String>,

        /// Controls whether to graph the diary after the fill.
        #[structopt(long)]
        no_graph: bool,
    },

    /// Displays the habit data according to the specified options to the terminal.
    Graph {
        /// Start the graphing from this date. Must be in format YYYY-MM-DD.
        #[structopt(long)]
        from_date: Option<String>,
    },

    /// Queries for habit information on the specified date.
    Insert {
        /// Date to set habit data on.  Must be in format YYYY-MM-DD.
        date: String,

        /// Controls whether to graph the diary after the fill.
        #[structopt(long)]
        no_graph: bool,
    },

    /// Prints the persistent configuration.
    ListConfig,

    /// Provide a comma separated list of habit categories. A new diary file is created at the specified
    /// --datafile path.
    New { category_list: String },

    /// Saves the specified options to persistent configuration.
    SaveConfig,
}

fn main() -> Result<()> {
    let opt = handle_config()?;
    let datafile_path = opt.datafile.as_ref().unwrap();
    match opt.command {
        Command::Fill {
            ref from_date,
            no_graph,
        } => {
            let from_date = parse_from_date(from_date)?;
            let mut data = datafile::open_datafile(datafile_path)?;
            let to_date = get_graph_date(&*data)?;
            fill_datafile(datafile_path, &from_date, &to_date, &mut *data)?;
            if !no_graph {
                plot_datafile(&opt, &to_date, &*data)?;
            }
        }
        Command::Graph { ref from_date } => {
            let data = datafile::open_datafile(datafile_path)?;
            let from_date = parse_from_date(from_date)?;
            let from_date = from_date.unwrap_or_else(|| Local::today().naive_local());
            plot_datafile(&opt, &from_date, &*data)?;
        }
        Command::Insert { ref date, no_graph } => {
            let date = parse_from_date(&Some(date.clone()))?.unwrap();
            let mut data = datafile::open_datafile(datafile_path)?;
            insert_to_datafile(datafile_path, &date, &mut *data)?;
            if !no_graph {
                plot_datafile(&opt, &date, &*data)?;
            }
        }
        Command::ListConfig => {
            let persistent_config = configuration::load_config()?;
            println!(
                "Listing persistent configuration loaded from '{}'\n{}",
                configuration::get_config_path().to_string_lossy(),
                &persistent_config
            );
        }
        Command::New { ref category_list } => {
            create_new(datafile_path, category_list)?;
        }
        Command::SaveConfig => {
            save_config(&opt)?;
        }
    }

    Ok(())
}

fn handle_config() -> Result<CliOptions> {
    let opt = CliOptions::from_args();
    let persistent_config = configuration::load_config()?;
    let opt = merge_cli_and_persistent_options(opt, &persistent_config);
    Ok(opt)
}

fn parse_from_date(input_date: &Option<String>) -> Result<Option<NaiveDate>> {
    match input_date {
        Some(date_string) => Ok(Some(
            NaiveDate::parse_from_str(date_string, datafile::DATE_FORMAT).context(format!(
                "Could not parse input date \"{}\". Use the format {}",
                date_string,
                datafile::DATE_FORMAT
            ))?,
        )),
        None => Ok(None),
    }
}

fn get_graph_date(data: &dyn DiaryDataConnection) -> Result<NaiveDate> {
    let today = Local::today().naive_local();
    if data.get_row(&today).is_some() {
        Ok(today)
    } else {
        Local::today()
            .naive_local()
            .checked_sub_signed(chrono::Duration::days(1))
            .ok_or_else(|| anyhow::Error::msg("Could not get yesterday"))
    }
}

fn merge_cli_and_persistent_options(
    options_from_cli: CliOptions,
    persistent_config: &configuration::Config,
) -> CliOptions {
    CliOptions {
        datafile: options_from_cli
            .datafile
            .or_else(|| Some(persistent_config.datafile_path.clone())),
        graph_days: options_from_cli
            .graph_days
            .or(Some(persistent_config.graph_days)),
        past_periods: options_from_cli
            .past_periods
            .or(Some(persistent_config.past_periods)),
        max_displayed_cols: options_from_cli
            .max_displayed_cols
            .or(Some(persistent_config.max_displayed_cols)),
        list_previous_days: options_from_cli
            .list_previous_days
            .or(Some(persistent_config.list_previous_days)),
        ..options_from_cli
    }
}

fn save_config(opt: &CliOptions) -> Result<()> {
    let provided_datafile_path = opt
        .datafile
        .clone()
        .unwrap_or_else(configuration::get_default_datafile_path);
    let full_datafile_path = std::fs::canonicalize(provided_datafile_path.clone());
    if full_datafile_path.is_err() {
        println!("Cannot canonicalize provided datafile path, saving the uncanonicalized path to configuration");
    }
    let updated_config = configuration::Config {
        datafile_path: full_datafile_path.unwrap_or(provided_datafile_path),
        graph_days: opt.graph_days.unwrap_or(configuration::DEFAULT_GRAPH_DAYS),
        past_periods: opt
            .past_periods
            .unwrap_or(configuration::DEFAULT_PAST_PERIODS),
        max_displayed_cols: opt
            .max_displayed_cols
            .unwrap_or(configuration::DEFAULT_MAX_DISPLAYED_COLS),
        list_previous_days: opt
            .list_previous_days
            .unwrap_or(configuration::DEFAULT_LIST_PREVIOUS_DAYS),
    };
    configuration::save_config(&updated_config)?;
    println!("Successfully updated persistent configuration");
    Ok(())
}

fn input_day_interactively(data: &mut dyn DiaryDataConnection, date: &NaiveDate) -> Result<()> {
    let prompt = format!(
        "Enter habit data for date {}",
        date.format(datafile::DATE_FORMAT)
    );
    let header = data.get_header();
    let selected_items = MultiSelect::new()
        .with_prompt(prompt)
        .items(header)
        .interact()?;

    let mut append_bools = vec![false; header.len()];
    for idx in selected_items {
        append_bools[idx] = true;
    }

    data.update_data(date, &append_bools)?;
    Ok(())
}

fn fill_datafile(
    datafile_path: &Path,
    from_date: &Option<NaiveDate>,
    to_date: &NaiveDate,
    data: &mut dyn DiaryDataConnection,
) -> Result<()> {
    let mut missing_dates = data.get_missing_dates(from_date, to_date);
    if data.is_empty() {
        missing_dates.push(
            Local::today()
                .naive_local()
                .checked_sub_signed(Duration::days(1))
                .unwrap(),
        );
    }
    for date in missing_dates {
        input_day_interactively(data, &date)?;
    }
    data.serialize(datafile_path)?;
    Ok(())
}

fn insert_to_datafile(
    datafile_path: &Path,
    date: &NaiveDate,
    data: &mut dyn DiaryDataConnection,
) -> Result<()> {
    input_day_interactively(data, date)?;
    data.serialize(datafile_path)?;
    Ok(())
}

fn plot_datafile(
    opt: &CliOptions,
    last_date: &NaiveDate,
    data: &dyn DiaryDataConnection,
) -> Result<()> {
    if opt.list_previous_days.unwrap() > 0 {
        let start_day =
            *last_date - chrono::Duration::days(opt.list_previous_days.unwrap() as i64 - 1i64);
        print!(
            "{}",
            graphing::pretty_print_diary_rows(data, &start_day, last_date)
        );
    }
    graphing::graph_last_n_days(
        data,
        last_date,
        opt.graph_days.unwrap(),
        opt.past_periods.unwrap(),
        opt.max_displayed_cols.unwrap(),
    )?;
    Ok(())
}

fn create_new(path: &Path, headers_string: &str) -> Result<()> {
    let mut headers_vector = vec![];
    for title in headers_string.split(',') {
        if title.is_empty() {
            bail!(format!("Invalid header specification: {}", headers_string));
        }
        headers_vector.push(String::from(title));
    }
    datafile::create_new_csv(path, &headers_vector)?;
    println!("New datafile successfully created at {}", path.display());
    Ok(())
}
