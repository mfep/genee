mod datafile;

use crate::datafile::parse_csv_to_diary_data;
use human_panic::setup_panic;
use structopt::StructOpt;
use std::path::PathBuf;

#[derive(StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    data_file: PathBuf,
}

fn main() {
    setup_panic!();
    let opt = Opt::from_args();
    let data = parse_csv_to_diary_data(&opt.data_file).unwrap();
    println!("{:?}", data);
}
