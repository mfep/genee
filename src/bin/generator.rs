use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use genee::datafile;
use rand::prelude::*;
use std::char;
use std::path::PathBuf;
use structopt::StructOpt;

const A_IDX: u32 = b'A' as u32;
const Z_IDX: u32 = b'Z' as u32;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,

    #[structopt(short, long)]
    rows: usize,

    #[structopt(short, long)]
    cols: usize,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let headers = generate_header(opt.cols);
    datafile::create_new_datafile(&opt.file, &headers)?;
    let mut data = datafile::open_datafile(&opt.file)?;
    data.update_data_batch(&generate_data(opt.cols, opt.rows))?;
    Ok(())
}

fn generate_header(cols: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut header = vec![];
    for _col in 0..cols {
        let rand_char = A_IDX + rng.next_u32() % (Z_IDX - A_IDX);
        header.push(String::from(char::from_u32(rand_char).unwrap()));
    }
    header
}

fn generate_data(cols: usize, rows: usize) -> Vec<(NaiveDate, Vec<usize>)> {
    let mut rng = rand::thread_rng();
    let mut data = vec![];
    for row in 0..rows {
        let mut row_data = vec![];
        for col in 1..cols + 1 {
            if rng.gen_bool(0.5) {
                row_data.push(col)
            }
        }
        let date =
            Local::now().naive_local() + Duration::try_days(1 + row as i64 - rows as i64).unwrap();
        data.push((date.date(), row_data));
    }
    data
}
