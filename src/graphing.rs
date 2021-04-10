use crate::datafile;
use crate::datafile::DiaryData;
use anyhow::{bail, Result};
use chrono::{Duration, Local};
use yansi::Paint;

pub fn graph_last_n_days(data: &DiaryData, n: usize, max_width: usize) -> Result<()> {
    if max_width < 10 {
        bail!("Graph height must be at least 10");
    }
    let today = Local::today().naive_local();
    let from_day = today.checked_sub_signed(Duration::days(n as i64)).unwrap();
    let counts = datafile::calculate_data_counts(data, &from_day, &today);
    let rows = generate_rows(&data.header, &counts, max_width)?;
    println!("{}", rows);
    Ok(())
}

fn generate_rows(names: &[String], counts: &[usize], max_width: usize) -> Result<String> {
    const BLOCK: &str = "▇";
    if names.len() != counts.len() {
        bail!("Input header length does not match count length");
    }
    let mut ret = String::new();
    let max_width = max_width - 8;
    let max_count = counts.iter().max();
    if max_count.is_none() {
        bail!("No input data");
    }
    let max_count = max_count.unwrap();
    for (name, &count) in names.iter().zip(counts.iter()) {
        ret += &format!("{:<3.3}", Paint::blue(name).italic());
        ret += " ";
        let width = count * max_width / max_count;
        if width == 0 {
            ret += &Paint::green("▏").to_string();
            ret += &format!("{}\n", Paint::new(count).bold());
        } else {
            for _ in 0..width {
                ret += &Paint::green(BLOCK).to_string();
            }
            ret += &format!(" {}\n", Paint::new(count).bold());
        }
    }
    Ok(ret)
}
