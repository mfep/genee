use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
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
    for line in reader.lines() {
        let line = line.context("Cannot read data file")?;
        let mut splitted = line.split(DELIMETER);
        let date_str = splitted
            .nth(0)
            .context("Date does not exist in data file")?;
        let mut row = DiaryRow {
            date: NaiveDate::parse_from_str(date_str, DATE_FORMAT)
                .context(format!("Cannot parse date in data file: \"{}\"", date_str))?,
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
    for header_str in header_line.split(DELIMETER) {
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
