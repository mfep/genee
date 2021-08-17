use anyhow::{bail, Context, Result};
use chrono::Local;
use chrono::NaiveDate;
use genee::configuration;
use genee::datafile;
use genee::datafile::DiaryData;
use genee::graphing;
use std::io;
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

    /// If set, habit information for all the missing days is queried between --append-date
    /// and yesterday. If --append-date is not set, all the missing days are queried between the
    /// first entry in the diary and yesterday.
    #[structopt(short, long)]
    fill: bool,

    /// When provided, the habit data is queried and written to the diary at the specified date.
    /// The format of the date must be YYYY-MM-DD.
    /// If --fill is also set, this option serves a different purpose.
    #[structopt(short, long)]
    append_date: Option<String>,

    /// Specifies the number of displayed periods when graphing the diary data.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(short, long)]
    past_periods: Option<usize>,

    /// Specifies the maximum allowed width of the terminal output.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(long)]
    max_displayed_cols: Option<usize>,

    /// If set, the current persistent configuration is displayed to the terminal.
    #[structopt(long)]
    list_config: bool,

    /// If set, the provided values for --datafile --graph-days --past-periods and --max-displayed-cols
    /// options are written to the persistent configuration.
    #[structopt(long)]
    save_config: bool,

    /// Provide a comma separated list of habit categories. A new diary file is created at the specified
    /// --datafile path. Be aware that this overwrites any existing diary file.
    #[structopt(long)]
    new: Option<String>,
}

fn main() -> Result<()> {
    let persistent_config = configuration::load_config()?;
    let opt = merge_cli_and_persistent_options(&persistent_config);
    if opt.save_config {
        save_config(&opt)?;
    }
    if opt.list_config {
        pretty_print_config()?;
        return Ok(());
    }
    let datafile_path = opt.datafile.unwrap();
    if opt.new.is_some() {
        create_new(&datafile_path, opt.new.as_ref().unwrap())?;
    }
    let mut data = datafile::parse_csv_to_diary_data(&datafile_path)?;
    let append_date = get_append_date(&opt.append_date)?;
    let graph_date: NaiveDate;
    if opt.fill {
        graph_date = yesterday();
        let appended_dates = datafile::get_missing_dates(&data, &append_date, &graph_date)?;
        for date in appended_dates {
            update_data(&mut data, &date)?;
        }
        datafile::serialize_to_csv(&datafile_path, &data)?;
    } else if append_date.is_some() {
        graph_date = append_date.unwrap();
        update_data(&mut data, &graph_date)?;
        datafile::serialize_to_csv(&datafile_path, &data)?;
    } else {
        graph_date = Local::today().naive_local();
    }
    graphing::graph_last_n_days(
        &data,
        &graph_date,
        opt.graph_days.unwrap(),
        opt.past_periods.unwrap(),
        opt.max_displayed_cols.unwrap(),
    )?;
    Ok(())
}

fn get_append_date(input_date: &Option<String>) -> Result<Option<NaiveDate>> {
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

fn merge_cli_and_persistent_options(persistent_config: &configuration::Config) -> CliOptions {
    let options_from_cli = CliOptions::from_args();
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
        ..options_from_cli
    }
}

fn pretty_print_config() -> Result<()> {
    let persistent_config = configuration::load_config()?;
    println!(
        "Listing persistent configuration loaded from \"{}\"\n{}",
        configuration::get_config_path().to_string_lossy(),
        configuration::pretty_print_config(&persistent_config)?
    );
    Ok(())
}

fn save_config(opt: &CliOptions) -> Result<()> {
    let provided_datafile_path = opt.datafile.clone().unwrap();
    let full_datafile_path = std::fs::canonicalize(provided_datafile_path.clone());
    if full_datafile_path.is_err() {
        println!("Cannot canonicalize provided datafile path, saving the uncanonicalized path to configuration");
    }
    let updated_config = configuration::Config {
        datafile_path: full_datafile_path.unwrap_or(provided_datafile_path),
        graph_days: opt.graph_days.unwrap(),
        past_periods: opt.past_periods.unwrap(),
        max_displayed_cols: opt.max_displayed_cols.unwrap(),
    };
    configuration::save_config(&updated_config)?;
    println!("Successfully updated persistent configuration");
    Ok(())
}

fn update_data(data: &mut DiaryData, date: &NaiveDate) -> Result<()> {
    let append_bools = input_data_interactively(date, &data.header);
    match datafile::update_data(data, date, &append_bools)? {
        datafile::SuccessfulUpdate::AddedNew => {
            println!(
                "Adding new row to datafile:\n{}",
                graphing::pretty_print_diary_row(data, date)
            );
            Ok(())
        }
        datafile::SuccessfulUpdate::ReplacedExisting(_existing_row) => {
            println!(
                "Updated row in datafile:\n{}",
                graphing::pretty_print_diary_row(data, date)
            );
            Ok(())
        }
    }
}

fn input_data_interactively(date: &NaiveDate, headers: &[String]) -> Vec<bool> {
    println!(
        "Enter habit data for date {}",
        date.format(datafile::DATE_FORMAT)
    );
    headers
        .iter()
        .map(|header| {
            println!("{} ?", header);
            let mut line = String::new();
            let _count = io::stdin().read_line(&mut line);
            !line.trim().is_empty()
        })
        .collect()
}

fn yesterday() -> NaiveDate {
    Local::today()
        .naive_local()
        .checked_sub_signed(chrono::Duration::days(1))
        .unwrap()
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
    Ok(())
}
