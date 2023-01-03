//! Structures and functions related to parsing and processing
//! CSV files that contain habit data
use super::{DiaryDataConnection, SuccessfulUpdate};
use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

/// Delimeter character used in the CSV data files.
const DELIMETER: char = ',';

/// Format of the date string in the CSV data file.
/// For example: 2020-01-25
pub const DATE_FORMAT: &str = "%Y-%m-%d";

/// A complete in-memory representation of the CSV data file.
#[derive(Debug, Default)]
struct DiaryDataCsv {
    /// Header of the data file, containing the names/abbreviations of the tracked habits.
    pub header: Vec<String>,

    /// Entries in the data file.
    pub data: BTreeMap<NaiveDate, Vec<bool>>,
}

/// Reads a CSV datafile to memory and returns the result boxed.
pub fn open_csv_datafile(path: &Path) -> Result<Box<dyn DiaryDataConnection>> {
    let mut reader = get_datafile_reader(path)?;
    let mut data = DiaryDataCsv {
        header: read_header(&mut reader)?,
        data: BTreeMap::default(),
    };
    for (i, line) in reader.lines().enumerate() {
        let line = line.context("Cannot read data file")?;
        let mut splitted = line.split(DELIMETER);
        let date_str = splitted
            .next()
            .context("Date does not exist in data file")?;
        let current_date = NaiveDate::parse_from_str(date_str, DATE_FORMAT)
            .context(format!("Cannot parse date in data file: \"{}\"", date_str))?;
        if data.data.contains_key(&current_date) {
            bail!(format!(
                "Data file contains duplicated date at line {}. Please fix manually!",
                i + 2
            ));
        }
        let mut row_data = vec![];
        for part in splitted {
            let part = part.trim();
            row_data.push(!part.is_empty());
        }
        if row_data.len() != data.header.len() {
            bail!(format!(
                "Number of entries ({}) on line {} in datafile does not match number of entries in the header ({})",
                row_data.len(),
                i + 2,
                data.header.len()));
        }
        data.data.insert(current_date, row_data);
    }
    Ok(Box::new(data))
}

/// Calculates the occurences of all habits in the prescribed date interval.
/// Both limits are inclusive.
fn calculate_data_counts(data: &DiaryDataCsv, from: &NaiveDate, to: &NaiveDate) -> Vec<usize> {
    let mut result: Vec<usize> = data.header.iter().map(|_| 0).collect();
    for (date, data) in data.data.iter().rev() {
        if date < from || date > to {
            continue;
        }
        for (i, &val) in data.iter().enumerate() {
            if val {
                result[i] += 1;
            }
        }
    }
    result
}

impl DiaryDataConnection for DiaryDataCsv {
    fn calculate_data_counts_per_iter(
        &self,
        date_ranges: &[(NaiveDate, NaiveDate)],
    ) -> Vec<Vec<usize>> {
        date_ranges
            .iter()
            .map(|(start_date, end_date)| calculate_data_counts(self, end_date, start_date))
            .collect()
    }

    fn update_data(&mut self, date: &NaiveDate, new_row: &[bool]) -> Result<SuccessfulUpdate> {
        if self.header.len() != new_row.len() {
            bail!("The provided update row does not match the datafile header in size");
        }
        match self.data.insert(*date, new_row.to_vec()) {
            Some(replaced_row) => Ok(SuccessfulUpdate::ReplacedExisting(replaced_row)),
            None => Ok(SuccessfulUpdate::AddedNew),
        }
    }

    fn serialize(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path).context("Could not open file for writing")?;
        let header = self.header.join(&String::from(DELIMETER));
        writeln!(file, "date,{}", header)?;
        for (date, data) in &self.data {
            writeln!(file, "{}", serialize_row(date, data))?;
        }
        Ok(())
    }

    fn get_missing_dates(&self, from: &Option<NaiveDate>, until: &NaiveDate) -> Vec<NaiveDate> {
        if from.is_none() && self.data.is_empty() {
            return vec![];
        }
        let first_date = from.unwrap_or_else(|| *self.data.iter().next().unwrap().0);
        let mut result = vec![];
        let mut date_to_check = first_date;
        while date_to_check <= *until {
            if !self.data.contains_key(&date_to_check) {
                result.push(date_to_check);
            }
            date_to_check = date_to_check
                .checked_add_signed(chrono::Duration::days(1))
                .unwrap();
        }
        result
    }

    fn get_header(&self) -> &[String] {
        &self.header
    }

    fn get_row(&self, date: &NaiveDate) -> Option<&Vec<bool>> {
        self.data.get(date)
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Formats a data row with a date to `String`.
fn serialize_row(date: &NaiveDate, data: &[bool]) -> String {
    let formatted_date = date.format(DATE_FORMAT);
    let content: Vec<&str> = data.iter().map(|&x| if x { "x" } else { "" }).collect();
    let joined_content = content.join(&String::from(DELIMETER));
    format!("{}{}{}", formatted_date, DELIMETER, joined_content)
}

/// Creates a new CSV data file at the specified path from a header list.
pub fn create_new_csv(path: &Path, headers: &[String]) -> Result<()> {
    let data = DiaryDataCsv {
        header: headers.to_vec(),
        data: BTreeMap::default(),
    };
    if path.exists() {
        bail!(format!("A file already exists at \"{}\"", path.display()))
    }
    data.serialize(path)?;
    Ok(())
}

fn get_datafile_reader(path: &Path) -> Result<BufReader<File>> {
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

#[test]
fn test_calculate_data_counts_per_iter() {
    let mut data = DiaryDataCsv {
        header: vec![String::from("A"), String::from("B"), String::from("C")],
        data: BTreeMap::default(),
    };
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 1), vec![true, false, false]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 2), vec![true, false, false]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 3), vec![true, true, false]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 4), vec![true, true, true]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 5), vec![true, false, false]);
    let ranges = super::get_date_ranges(&NaiveDate::from_ymd(2021, 1, 5), 2, 3);
    let result = data.calculate_data_counts_per_iter(&ranges);
    assert_eq!(vec![vec![2, 1, 1], vec![2, 1, 0], vec![1, 0, 0],], result);
}

#[test]
fn test_calculate_data_counts() {
    let mut data = DiaryDataCsv {
        header: vec![String::from("A"), String::from("B"), String::from("C")],
        data: BTreeMap::default(),
    };
    data.data
        .insert(NaiveDate::from_ymd(2020, 1, 1), vec![true, false, false]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 1), vec![true, false, false]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 2), vec![true, true, false]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 3), vec![true, true, true]);
    data.data
        .insert(NaiveDate::from_ymd(2021, 1, 4), vec![true, false, false]);
    let result = calculate_data_counts(
        &data,
        &NaiveDate::from_ymd(2020, 8, 5),
        &NaiveDate::from_ymd(2021, 1, 3),
    );
    assert_eq!(vec![3, 2, 1], result);
}
