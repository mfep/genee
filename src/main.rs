use anyhow::{Context, Result};
use chrono::Local;
use chrono::NaiveDate;
use genee::datafile;
use genee::graphing;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,

    #[structopt(short, long, default_value = "30")]
    graph_days: usize,

    #[structopt(short, long)]
    append: Option<String>,

    #[structopt(short, long)]
    date_to_append: Option<String>,

    #[structopt(short, long, default_value = "2")]
    past_periods: usize,

    #[structopt(short, long, default_value = "60")]
    width: usize,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let mut data = datafile::parse_csv_to_diary_data(&opt.file)?;
    if opt.append.is_some() {
        let append_bools = parse_appendee(&opt.append.unwrap());
        let append_date = get_append_date(&opt.date_to_append)?;
        match datafile::update_data(&mut data, &append_date, &append_bools)? {
            datafile::SuccessfulUpdate::AddedNew => (),
            datafile::SuccessfulUpdate::ReplacedExisting(existing_row) => {
                println!(
                    "Updating row in datafile: {}",
                    datafile::serialize_row(&append_date, &existing_row)
                )
            }
        }
        datafile::serialize_to_csv(&opt.file, &data)?;
    }
    graphing::graph_last_n_days(&data, opt.graph_days, opt.past_periods, opt.width)?;
    Ok(())
}

fn parse_appendee(appendee: &str) -> Vec<bool> {
    appendee
        .split(datafile::DELIMETER)
        .map(|s| !s.is_empty())
        .collect()
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
