use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

const DELIMETER: char = ',';

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
    let csv_file = File::open(path).context(format!("Cannot open data file at {:?}", path))?;
    let mut reader = BufReader::new(csv_file);
    let mut data = DiaryData::default();

    let mut header_line = String::new();
    reader
        .read_line(&mut header_line)
        .context("Cannot read first line of data file")?;
    for header_str in header_line.split(DELIMETER) {
        let header_str = header_str.trim();
        if header_str.is_empty() {
            bail!("Data file header is empty");
        }
        data.header.push(String::from(header_str));
    }
    for line in reader.lines() {
        let line = line.context("Cannot read data file")?;
        let mut splitted = line.split(DELIMETER);
        let date_str = splitted
            .nth(0)
            .context("Date does not exist in data file")?;
        let mut row = DiaryRow {
            date: NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
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
    let mut result : Vec<usize> = data.header.iter().map(|_| 0).collect();
    for row in data.data.iter().rev() {
        let date = &row.date;
        if date < &from || date > &to {
            continue;
        }
        for (i, val) in row.data.iter().enumerate() {
            if *val {
                result[i] += 1;
            }
        }
    }
    return result;
}

#[test]
fn test_calculate_data_counts() {
    let data = DiaryData {
        header: vec![
            String::from("A"),
            String::from("B"),
            String::from("C")],
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
    let result = calculate_data_counts(&data, &NaiveDate::from_ymd(2020, 8, 5), &NaiveDate::from_ymd(2021, 1, 3));
    assert_eq!(vec![3, 2, 1], result);
}
