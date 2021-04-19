use anyhow::Result;
use chrono::Local;
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

    #[structopt(short, long, default_value = "2")]
    past_periods: usize,

    #[structopt(short, long, default_value = "60")]
    width: usize,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    if opt.append.is_some() {
        let append_bools = parse_appendee(&opt.append.unwrap());
        datafile::append_data_to_datafile(&opt.file, &Local::today().naive_local(), &append_bools)?;
    }
    let data = datafile::parse_csv_to_diary_data(&opt.file)?;
    graphing::graph_last_n_days(&data, opt.graph_days, opt.past_periods, opt.width)?;
    Ok(())
}

fn parse_appendee(appendee: &str) -> Vec<bool> {
    appendee
        .split(datafile::DELIMETER)
        .map(|s| !s.is_empty())
        .collect()
}
