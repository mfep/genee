use anyhow::Result;
use chrono::Local;
use genee::datafile;
use genee::graphing;
use human_panic::setup_panic;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,

    #[structopt(short, long)]
    graph_days: Option<usize>,

    #[structopt(short, long)]
    append: Option<String>,
}

fn main() {
    setup_panic!();
    let opt = Opt::from_args();
    if opt.append.is_some() {
        let append_bools = parse_appendee(&opt.append.unwrap()).unwrap();
        datafile::append_data_to_datafile(&opt.file, &Local::today().naive_local(), &append_bools)
            .unwrap();
    }
    if opt.graph_days.is_some() {
        let data = datafile::parse_csv_to_diary_data(&opt.file).unwrap();
        graphing::graph_last_n_days(&data, opt.graph_days.unwrap(), 50).unwrap();
    }
}

fn parse_appendee(appendee: &str) -> Result<Vec<bool>> {
    Ok(appendee
        .split(datafile::DELIMETER)
        .map(|s| !s.is_empty())
        .collect())
}
