use anyhow::{Context, Result};
use chrono::Local;
use chrono::NaiveDate;
use genee::configuration;
use genee::datafile;
use genee::graphing;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct CliOptions {
    #[structopt(short, long, parse(from_os_str))]
    file: Option<PathBuf>,

    #[structopt(short, long)]
    graph_days: Option<usize>,

    #[structopt(short, long)]
    append: bool,

    #[structopt(short, long)]
    date_to_append: Option<String>,

    #[structopt(short, long)]
    past_periods: Option<usize>,

    #[structopt(long)]
    max_displayed_cols: Option<usize>,

    #[structopt(long)]
    list_config: bool,

    #[structopt(long)]
    save_config: bool,
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
    let mut data = datafile::parse_csv_to_diary_data(&opt.file.as_ref().unwrap())?;
    let append_date = get_append_date(&opt.date_to_append)?;
    if opt.append {
        let append_bools = graphing::input_data_interactively(&append_date, &data.header);
        match datafile::update_data(&mut data, &append_date, &append_bools)? {
            datafile::SuccessfulUpdate::AddedNew => {
                println!(
                    "Adding new row to datafile: {}",
                    datafile::serialize_row(&append_date, &append_bools)
                )
            }
            datafile::SuccessfulUpdate::ReplacedExisting(_existing_row) => {
                println!(
                    "Updated row in datafile: {}",
                    datafile::serialize_row(&append_date, &append_bools)
                )
            }
        }
        datafile::serialize_to_csv(&opt.file.unwrap(), &data)?;
    }
    graphing::graph_last_n_days(
        &data,
        &append_date,
        opt.graph_days.unwrap(),
        opt.past_periods.unwrap(),
        opt.max_displayed_cols.unwrap(),
    )?;
    Ok(())
}

fn get_append_date(input_date: &Option<String>) -> Result<NaiveDate> {
    match input_date {
        Some(date_string) => Ok(
            NaiveDate::parse_from_str(&date_string, datafile::DATE_FORMAT).context(format!(
                "Could not parse input data string \"{}\"",
                date_string
            ))?,
        ),
        None => Ok(Local::today().naive_local()),
    }
}

fn merge_cli_and_persistent_options(persistent_config: &configuration::Config) -> CliOptions {
    let options_from_cli = CliOptions::from_args();
    CliOptions {
        file: options_from_cli
            .file
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
    let updated_config = configuration::Config {
        datafile_path: std::fs::canonicalize(opt.file.as_ref().unwrap())?,
        graph_days: opt.graph_days.unwrap(),
        past_periods: opt.past_periods.unwrap(),
        max_displayed_cols: opt.max_displayed_cols.unwrap(),
    };
    configuration::save_config(&updated_config)?;
    println!("Successfully updated persistent configuration");
    Ok(())
}
