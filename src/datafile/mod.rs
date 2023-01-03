//! Handling of habit databases.
mod csv_datafile;
use anyhow::Result;
use chrono::{Duration, NaiveDate};
use std::path::Path;

/// Format of the dates used in the program.
pub const DATE_FORMAT: &str = csv_datafile::DATE_FORMAT;

/// Result of an update to a `DiaryDataConnection` instance.
pub enum SuccessfulUpdate {
    /// The new date was not present in the instance, but it was added.
    AddedNew,

    /// The date was already present in the instance, but was replaced.
    /// This element contains the original data row.
    ReplacedExisting(Vec<bool>),
}

/// Represents a connection to the diary database.
pub trait DiaryDataConnection {
    /// Calculates the occurences of all habits over multiple periods of date ranges.
    fn calculate_data_counts_per_iter(
        &self,
        date_ranges: &[(NaiveDate, NaiveDate)],
    ) -> Vec<Vec<usize>>;

    /// Modifies the provided `DiaryDataConnection` instance with the provided data row and date.
    fn update_data(&mut self, date: &NaiveDate, new_row: &[bool]) -> Result<SuccessfulUpdate>;

    /// Returns a vector of missing dates between the first date in the database until specified date.
    fn get_missing_dates(&self, from: &Option<NaiveDate>, until: &NaiveDate) -> Vec<NaiveDate>;

    /// Get the list of habits tracked by the database.
    fn get_header(&self) -> &[String];

    /// Get the habit data for a particular date, if exists, from the database.
    fn get_row(&self, date: &NaiveDate) -> Option<&Vec<bool>>;

    /// Returns if the database contains any records.
    fn is_empty(&self) -> bool;
}

/// Tries to read data file to memory.
pub fn open_datafile(path: &Path) -> Result<Box<dyn DiaryDataConnection>> {
    let diary_csv = csv_datafile::open_csv_datafile(path)?;
    Ok(diary_csv)
}

/// Calculates the date ranges according to the parameters.
/// For example when `range_size == 30`, `iters == 3` and `from_date` is today,
/// the result is a 3-element vector containing ranges of the last 30 days,
/// the 30 days before that, and the 30 days before the latter one.
pub fn get_date_ranges(
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

/// Create a new database on the prescribed path, using the prescribed headers.
pub fn create_new_datafile(path: &Path, headers: &[String]) -> Result<()> {
    csv_datafile::create_new_csv(path, headers)?;
    Ok(())
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
