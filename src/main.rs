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

    /// Specifies the number of days from the diary that should be printed in a tabular format.
    #[structopt(short, long)]
    list_previous_days: Option<usize>,

    /// Specifies the maximum allowed width of the terminal output.
    /// When not provided, its value is loaded from persistent configuration file.
    #[structopt(long)]
    max_displayed_cols: Option<usize>,

    /// If set, the current persistent configuration is displayed to the terminal.
    #[structopt(long)]
    list_config: bool,

    /// If set, the provided values for --datafile --graph-days --past-periods --max-displayed-cols
    /// and --list-previous-days options are written to the persistent configuration.
    /// Unspecified options are reset to their default value.
    #[structopt(long)]
    save_config: bool,

    /// Provide a comma separated list of habit categories. A new diary file is created at the specified
    /// --datafile path. Be aware that this overwrites any existing diary file.
    #[structopt(long)]
    new: Option<String>,
}

fn main() -> Result<()> {
    let opt = handle_config()?;
    let datafile_path = opt.datafile.as_ref().unwrap();
    if opt.new.is_some() {
        create_new(datafile_path, opt.new.as_ref().unwrap())?;
    }
    let mut data = datafile::parse_csv_to_diary_data(datafile_path)?;

    let append_date = get_append_date(&opt.append_date)?;
    let last_date = get_graph_date(&data)?;

    data = modify_datafile(&opt, &append_date, &last_date, data)?;
    plot_datafile(&opt, &last_date, &data)?;

    Ok(())
}

fn handle_config() -> Result<CliOptions> {
    let opt = CliOptions::from_args();
    if opt.save_config {
        save_config(&opt)?;
    }
    let persistent_config = configuration::load_config()?;
    let opt = merge_cli_and_persistent_options(opt, &persistent_config);
    if opt.list_config {
        println!(
            "Listing persistent configuration loaded from \"{}\"\n{}",
            configuration::get_config_path().to_string_lossy(),
            &persistent_config
        );
    }
    Ok(opt)
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

fn get_graph_date(data: &datafile::DiaryData) -> Result<NaiveDate> {
    let today = Local::today().naive_local();
    if data.data.contains_key(&today) {
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

fn input_day_interactively(data: &mut DiaryData, date: &NaiveDate) -> Result<()> {
    println!(
        "Enter habit data for date {}",
        date.format(datafile::DATE_FORMAT)
    );
    let append_bools: Vec<bool> = data
        .header
        .iter()
        .map(|header| {
            println!("{} ?", header);
            let mut line = String::new();
            let _count = io::stdin().read_line(&mut line);
            !line.trim().is_empty()
        })
        .collect();
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

fn modify_datafile(
    opt: &CliOptions,
    append_date: &Option<NaiveDate>,
    last_date: &NaiveDate,
    mut data: DiaryData,
) -> Result<DiaryData> {
    if opt.fill {
        let appended_dates = datafile::get_missing_dates(&data, append_date, last_date)?;
        for date in appended_dates {
            input_day_interactively(&mut data, &date)?;
        }
    } else if let Some(date) = *append_date {
        input_day_interactively(&mut data, &date)?;
    }
    datafile::serialize_to_csv(opt.datafile.as_ref().unwrap(), &data)?;
    Ok(data)
}

fn plot_datafile(opt: &CliOptions, last_date: &NaiveDate, data: &DiaryData) -> Result<()> {
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
    Ok(())
}
