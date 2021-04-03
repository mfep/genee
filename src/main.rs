mod datafile;
mod graphing;

use crate::datafile::parse_csv_to_diary_data;
use crate::graphing::graph_last_n_days;
use human_panic::setup_panic;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    data_file: PathBuf,
}

fn main() {
    setup_panic!();
    let opt = Opt::from_args();
    let data = parse_csv_to_diary_data(&opt.data_file).unwrap();
    graph_last_n_days(&data, 30, 50).unwrap();
}
