use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDate};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

pub const DELIMETER: char = ',';
const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug)]
pub struct DiaryRow {
    pub date: NaiveDate,
    pub data: Vec<bool>,
}

#[derive(Debug, Default)]
pub struct DiaryData {
    pub header: Vec<String>,
    pub data: Vec<DiaryRow>,
}

pub fn parse_csv_to_diary_data(path: &PathBuf) -> Result<DiaryData> {
    let mut reader = get_datafile_reader(path)?;
    let mut data = DiaryData::default();

    data.header = read_header(&mut reader)?;
    let mut last_date = NaiveDate::from_ymd(1, 1, 1);
    for (i, line) in reader.lines().enumerate() {
        let line = line.context("Cannot read data file")?;
        let mut splitted = line.split(DELIMETER);
        let date_str = splitted
            .nth(0)
            .context("Date does not exist in data file")?;
        let current_date = NaiveDate::parse_from_str(date_str, DATE_FORMAT)
            .context(format!("Cannot parse date in data file: \"{}\"", date_str))?;
        if current_date <= last_date {
            bail!(format!("Corrupt datestamp in datafile at line {}", i + 2));
        }
        last_date = current_date;
        let mut row = DiaryRow {
            date: current_date,
            data: vec![],
        };
        for part in splitted {
            let part = part.trim();
            row.data.push(!part.is_empty());
        }
        data.data.push(row);
    }
    Ok(data)
}

pub fn calculate_data_counts(data: &DiaryData, from: &NaiveDate, to: &NaiveDate) -> Vec<usize> {
    let mut result: Vec<usize> = data.header.iter().map(|_| 0).collect();
    for row in data.data.iter().rev() {
        let date = &row.date;
        if date < &from || date > &to {
            continue;
        }
        for (i, &val) in row.data.iter().enumerate() {
            if val {
                result[i] += 1;
            }
        }
    }
    return result;
}

pub fn calculate_data_counts_per_iter(
    data: &DiaryData,
    from: &NaiveDate,
    period: usize,
    iters: usize,
) -> Vec<Vec<usize>> {
    let periods = get_date_ranges(from, period, iters);
    periods
        .iter()
        .map(|(start_date, end_date)| calculate_data_counts(data, end_date, start_date))
        .collect()
}

pub fn append_data_to_datafile(path: &PathBuf, date: &NaiveDate, new_data: &[bool]) -> Result<()> {
    let header = read_header_only(path)?;
    if header.len() != new_data.len() {
        bail!("The provided additional data does not match the datafile header in size");
    }
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .context("Could not open datafile for writing")?;
    let date_string = date.format(DATE_FORMAT);
    let content: Vec<&str> = new_data.iter().map(|&x| if x { "x" } else { "" }).collect();
    let content = content.join(&String::from(DELIMETER));
    writeln!(file, "{}{}{}", date_string, DELIMETER, content)
        .context("Could not append to datafile")?;
    Ok(())
}

pub fn serialize_to_csv(path: &PathBuf, data: &DiaryData) -> Result<()> {
    let mut file = File::create(path).context("Could not open file for writing")?;
    let header = data.header.join(&String::from(DELIMETER));
    writeln!(file, "date,{}", header)?;
    for row in &data.data {
        let date = row.date.format(DATE_FORMAT);
        let content: Vec<&str> = row.data.iter().map(|&x| if x { "x" } else { "" }).collect();
        let joined_content = content.join(&String::from(DELIMETER));
        writeln!(file, "{}{}{}", date, DELIMETER, joined_content)?;
    }
    Ok(())
}

fn get_datafile_reader(path: &PathBuf) -> Result<BufReader<File>> {
    let csv_file = File::open(path).context(format!("Cannot open data file at {:?}", path))?;
    let reader = BufReader::new(csv_file);
    Ok(reader)
}

fn read_header(reader: &mut BufReader<File>) -> Result<Vec<String>> {
    let mut header_data = vec![];
    let mut header_line = String::new();
    reader
        .read_line(&mut header_line)
        .context("Cannot read first line of data file")?;
    for header_str in header_line.split(DELIMETER).skip(1) {
        // skip 'date'
        let header_str = header_str.trim();
        if header_str.is_empty() {
            bail!("Data file header is empty");
        }
        header_data.push(String::from(header_str));
    }
    Ok(header_data)
}

fn read_header_only(path: &PathBuf) -> Result<Vec<String>> {
    let mut reader = get_datafile_reader(path)?;
    read_header(&mut reader)
}

fn get_date_ranges(
    from_date: &NaiveDate,
    range_size: usize,
    iters: usize,
) -> Vec<(NaiveDate, NaiveDate)> {
    let start_offsets = (0..range_size * iters).step_by(range_size);
    let end_offsets = (range_size - 1..range_size * (iters + 1)).step_by(range_size);
    start_offsets
        .zip(end_offsets)
        .map(|(start, end)| {
            (
                *from_date - Duration::days(start as i64),
                *from_date - Duration::days(end as i64),
            )
        })
        .collect()
}

#[test]
fn test_calculate_data_counts_per_iter() {
    let data = DiaryData {
        header: vec![String::from("A"), String::from("B"), String::from("C")],
        data: vec![
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 1),
                data: vec![true, false, false],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 2),
                data: vec![true, false, false],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 3),
                data: vec![true, true, false],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 4),
                data: vec![true, true, true],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 5),
                data: vec![true, false, false],
            },
        ],
    };
    let result = calculate_data_counts_per_iter(&data, &NaiveDate::from_ymd(2021, 1, 5), 2, 3);
    assert_eq!(vec![vec![2, 1, 1], vec![2, 1, 0], vec![1, 0, 0],], result);
}

#[test]
fn test_get_date_ranges() {
    let result = get_date_ranges(&NaiveDate::from_ymd(2000, 5, 30), 5, 3);
    assert_eq!(
        vec![
            (
                NaiveDate::from_ymd(2000, 5, 30),
                NaiveDate::from_ymd(2000, 5, 26)
            ),
            (
                NaiveDate::from_ymd(2000, 5, 25),
                NaiveDate::from_ymd(2000, 5, 21)
            ),
            (
                NaiveDate::from_ymd(2000, 5, 20),
                NaiveDate::from_ymd(2000, 5, 16)
            ),
        ],
        result
    );
}

#[test]
fn test_calculate_data_counts() {
    let data = DiaryData {
        header: vec![String::from("A"), String::from("B"), String::from("C")],
        data: vec![
            DiaryRow {
                date: NaiveDate::from_ymd(2020, 1, 1),
                data: vec![true, false, false],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 1),
                data: vec![true, false, false],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 2),
                data: vec![true, true, false],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 3),
                data: vec![true, true, true],
            },
            DiaryRow {
                date: NaiveDate::from_ymd(2021, 1, 4),
                data: vec![true, false, false],
            },
        ],
    };
    let result = calculate_data_counts(
        &data,
        &NaiveDate::from_ymd(2020, 8, 5),
        &NaiveDate::from_ymd(2021, 1, 3),
    );
    assert_eq!(vec![3, 2, 1], result);
}
