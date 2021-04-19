use anyhow::Result;
use chrono::{Duration, Local};
use genee::datafile;
use genee::datafile::{DiaryData, DiaryRow};
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
    let generated_data = generate_data(opt.cols, opt.rows);
    datafile::serialize_to_csv(&opt.file, &generated_data)?;
    Ok(())
}

fn generate_data(cols: usize, rows: usize) -> DiaryData {
    let mut rng = rand::thread_rng();
    let mut header = vec![];
    for _col in 0..cols {
        let rand_char = A_IDX + rng.next_u32() % (Z_IDX - A_IDX);
        header.push(String::from(char::from_u32(rand_char).unwrap()));
    }
    let mut data = vec![];
    for row in 0..rows {
        let mut row_data = vec![];
        for _col in 0..cols {
            row_data.push(rng.gen_bool(0.5));
        }
        let date = Local::now().naive_local() + Duration::days(1 + row as i64 - rows as i64);
        data.push(DiaryRow {
            date: date.date(),
            data: row_data,
        });
    }
    DiaryData { header, data }
}
