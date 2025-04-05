//! Handling of habit databases.
use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDate, NaiveTime};
use std::{ffi::OsString, path::Path};

use rusqlite::{Connection, backup, params};

/// Format of the dates used in the program.
pub const DATE_FORMAT: &str = "%Y-%m-%d";

/// Result of an update to a `DiaryDataSqlite` instance.
pub enum SuccessfulUpdate {
    /// The new date was not present in the instance, but it was added.
    AddedNew,

    /// The date was already present in the instance, but was replaced.
    ReplacedExisting,
}

/// Result from the call to `add_category`
#[derive(Debug, PartialEq)]
pub enum AddCategoryResult {
    /// Created a new category
    AddedNew,

    /// Made a previously hidden category visible again
    Unhide,

    /// The category is already present and is visible
    AlreadyPresent,
}

/// Result from the call to `hide_category`
#[derive(Debug, PartialEq)]
pub enum HideCategoryResult {
    /// The specified category was visible previously and was hidden
    Hidden,

    /// The specified category is already hidden, nothing was changed
    AlreadyHidden,

    /// The specified category does not exist
    NonExistingCategory,
}

pub struct DiaryDataSqlite {
    connection: Connection,
}

const CURRENT_DB_VERSION: usize = 1;

fn insert_version_to_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO Info (info_name, info_value) VALUES (\"version\", ?1)",
        params![CURRENT_DB_VERSION],
    )?;
    Ok(())
}

fn initialize_sqlite_database(conn: &Connection, headers: &[String]) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
        DROP TABLE IF EXISTS Info;
        CREATE TABLE Info(
            info_id INTEGER PRIMARY KEY AUTOINCREMENT,
            info_name TEXT UNIQUE NOT NULL,
            info_value TEXT NOT NULL
        );
        DROP TABLE IF EXISTS Category;
        CREATE TABLE Category(
            category_id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            hidden INTEGER NOT NULL
        );
        DROP TABLE IF EXISTS DateEntry;
        CREATE TABLE DateEntry(
            date DATE PRIMARY KEY,
            created_at INTEGER NOT NULL
        );
        DROP TABLE IF EXISTS EntryToCategories;
        CREATE TABLE EntryToCategories(
            date INTEGER NOT NULL REFERENCES DateEntry(date) ON DELETE CASCADE,
            category_id INTEGER NOT NULL REFERENCES Category(category_id) ON DELETE CASCADE,
            PRIMARY KEY(category_id, date)
        );
        COMMIT;",
    )?;
    insert_version_to_db(conn)?;
    let now = chrono::Local::now().timestamp();
    for header in headers {
        conn.execute(
            "INSERT INTO Category (name, created_at, hidden) VALUES (?1, ?2, 0)",
            params![header, now],
        )?;
    }
    Ok(())
}

pub fn create_new_sqlite(path: &Path, headers: &[String]) -> Result<()> {
    let conn = Connection::open(path).context("Could not open/create SQLite database")?;
    initialize_sqlite_database(&conn, headers)?;
    Ok(())
}

fn open_sqlite_database(connection: Connection) -> Result<DiaryDataSqlite> {
    let data = DiaryDataSqlite { connection };
    let db_version = data.get_db_version()?;
    if db_version < CURRENT_DB_VERSION {
        println!(
            "Detected an SQLite datafile of version {}. Commencing update...",
            db_version
        );
        data.update_db()?;
    }
    Ok(data)
}

fn date_to_timestamp(date: &NaiveDate) -> i64 {
    date.and_time(NaiveTime::default()).and_utc().timestamp()
}

impl DiaryDataSqlite {
    pub fn into_any(self) -> Box<dyn std::any::Any> {
        Box::new(self)
    }

    pub fn calculate_data_counts_per_iter(
        &self,
        date_ranges: &[(NaiveDate, NaiveDate)],
    ) -> Result<Vec<Vec<usize>>> {
        let category_ids = self.get_visible_category_ids()?;
        let mut result = vec![];
        for (from, to) in date_ranges {
            result.push(self.calculate_data_counts(from, to, &category_ids)?);
        }
        Ok(result)
    }

    pub fn update_data(&mut self, date: &NaiveDate, new_row: &[usize]) -> Result<SuccessfulUpdate> {
        self.update_data_internal(&[(*date, new_row.to_vec())])
    }

    pub fn update_data_batch(&mut self, new_items: &[(NaiveDate, Vec<usize>)]) -> Result<()> {
        self.update_data_internal(new_items)?;
        Ok(())
    }

    pub fn get_missing_dates(
        &self,
        from: &Option<NaiveDate>,
        until: &NaiveDate,
    ) -> Result<Vec<NaiveDate>> {
        if self.is_empty()? {
            return Ok(vec![]);
        }

        // If no from, select the earliest date in the database
        let from = match from {
            Some(date) => *date,
            None => {
                let mut statement = self.connection.prepare("SELECT MIN(date) FROM DateEntry")?;
                let min_date = statement.query_row([], |row| row.get(0))?;
                DateTime::from_timestamp(min_date, 0).unwrap().date_naive()
            }
        };

        let mut statement = self
            .connection
            .prepare("SELECT date FROM DateEntry WHERE date>=?1 AND date<=?2")?;
        let from_timestamp = from.and_time(NaiveTime::default()).and_utc().timestamp();
        let until_timestamp = until.and_time(NaiveTime::default()).and_utc().timestamp();
        let rows = statement.query_map([from_timestamp, until_timestamp], |row| row.get(0))?;
        let mut missing_dates = vec![];

        let mut current_date = from;
        for date_val in rows {
            let next_present_day = DateTime::from_timestamp(date_val?, 0).unwrap().date_naive();
            while current_date <= *until {
                let last_date = current_date;
                current_date += chrono::Duration::try_days(1).unwrap();
                if next_present_day == last_date {
                    break;
                }
                missing_dates.push(last_date);
            }
        }
        while current_date <= *until {
            missing_dates.push(current_date);
            current_date += chrono::Duration::try_days(1).unwrap();
        }

        Ok(missing_dates)
    }

    pub fn get_header(&self) -> Result<Vec<(String, usize)>> {
        let mut statement = self.connection.prepare(
            "SELECT name, category_id FROM Category WHERE hidden=0 ORDER BY category_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<usize, String>(0)?, row.get::<usize, usize>(1)?))
        })?;
        let mut header = vec![];
        for row in rows {
            header.push(row?);
        }
        Ok(header)
    }

    pub fn get_row(&self, date: &NaiveDate) -> Result<Option<Vec<usize>>> {
        Ok(self.get_rows(date, date)?.pop().unwrap())
    }

    pub fn get_rows(&self, from: &NaiveDate, until: &NaiveDate) -> Result<Vec<Option<Vec<usize>>>> {
        let mut statement = self.connection.prepare(
            "SELECT date, group_concat(coalesce(category_id, 'EMPTY'), ';') FROM DateEntry
                LEFT JOIN EntryToCategories USING(date)
                WHERE date>=?1 AND date<=?2
                    AND (category_id ISNULL
                        OR 0=(SELECT hidden FROM Category WHERE EntryToCategories.category_id=Category.category_id))
                GROUP BY date
                ORDER BY date DESC")?;

        let mut rows =
            statement.query(params![date_to_timestamp(from), date_to_timestamp(until)])?;
        let mut results = vec![];
        let mut current_date = *until;
        while current_date >= *from {
            if let Some(row) = rows.next()? {
                let timestamp_s: i64 = row.get(0)?;
                let date = DateTime::from_timestamp(timestamp_s, 0)
                    .unwrap()
                    .date_naive();

                while date < current_date {
                    results.push(None);
                    current_date -= chrono::Duration::try_days(1).unwrap();
                }
                let row_data: String = row.get(1)?;
                if row_data == "EMPTY" {
                    results.push(Some(vec![]));
                } else {
                    let row_data_parsed = row_data
                        .split(';')
                        .map(|id| id.parse::<usize>().unwrap())
                        .collect();
                    results.push(Some(row_data_parsed));
                }
            } else {
                results.push(None);
            }
            current_date -= chrono::Duration::try_days(1).unwrap();
        }
        Ok(results)
    }

    pub fn is_empty(&self) -> Result<bool> {
        let mut statement = self.connection.prepare("SELECT COUNT(*) FROM DateEntry")?;
        let count: usize = statement.query_row([], |row| row.get(0))?;
        Ok(count == 0)
    }

    pub fn get_date_range(&self) -> Result<(NaiveDate, NaiveDate)> {
        if self.is_empty()? {
            bail!("Cannot get date range, datafile is empty")
        }
        let mut statement = self
            .connection
            .prepare("SELECT MIN(date), MAX(date) FROM DateEntry")?;
        let mut rows = statement.query([])?;
        let row = rows.next()?.unwrap();
        let min_date = DateTime::from_timestamp(row.get(0)?, 0)
            .unwrap()
            .date_naive();
        let max_date = DateTime::from_timestamp(row.get(1)?, 0)
            .unwrap()
            .date_naive();

        Ok((min_date, max_date))
    }

    pub fn add_category(&self, name: &str) -> Result<AddCategoryResult> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id, hidden FROM Category WHERE name=(?1)")?;
        let mut rows = statement.query(params![name])?;

        if let Some(row) = rows.next()? {
            let category_id: usize = row.get(0)?;
            let hidden = 0usize != row.get::<usize, usize>(1)?;

            if hidden {
                let mut statement = self
                    .connection
                    .prepare("UPDATE Category SET hidden=0 WHERE category_id=(?1)")?;
                statement.execute(params![category_id])?;
                Ok(AddCategoryResult::Unhide)
            } else {
                Ok(AddCategoryResult::AlreadyPresent)
            }
        } else {
            let mut statement = self
                .connection
                .prepare("INSERT INTO Category (name, created_at, hidden) VALUES (?1, ?2, 0)")?;
            let now = chrono::Local::now().timestamp();
            statement.execute(params![name, now])?;
            Ok(AddCategoryResult::AddedNew)
        }
    }

    pub fn hide_category(&self, name: &str) -> Result<HideCategoryResult> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id, hidden FROM Category WHERE name=(?1)")?;
        let mut rows = statement.query(params![name])?;
        if let Some(row) = rows.next()? {
            let category_id: usize = row.get(0)?;
            let hidden = 0usize != row.get::<usize, usize>(1)?;
            if hidden {
                Ok(HideCategoryResult::AlreadyHidden)
            } else {
                let mut statement = self
                    .connection
                    .prepare("UPDATE Category SET hidden=1 WHERE category_id=(?1)")?;
                statement.execute(params![category_id])?;
                Ok(HideCategoryResult::Hidden)
            }
        } else {
            Ok(HideCategoryResult::NonExistingCategory)
        }
    }

    pub fn get_most_frequent_daily_data(
        &self,
        from: &Option<NaiveDate>,
        until: &NaiveDate,
        max_count: Option<usize>,
    ) -> Result<Vec<(Vec<usize>, usize)>> {
        let from_timestamp = from
            .and_then(|from_date| Some(date_to_timestamp(&from_date)))
            .unwrap_or_default();
        let until_timestamp = date_to_timestamp(until);
        let max_count = max_count.unwrap_or(usize::MAX);

        let mut statement = self.connection.prepare(
        "SELECT concat_categories, COUNT(date) FROM (
            SELECT date, group_concat(category_id, ';') AS concat_categories FROM EntryToCategories WHERE date>=(?1) AND date<=(?2)
                AND 0=(SELECT hidden FROM Category WHERE EntryToCategories.category_id=Category.category_id)
            GROUP BY date
        ) GROUP BY concat_categories ORDER BY COUNT(date) DESC LIMIT (?3)
        ")?;
        let rows =
            statement.query_map(params![from_timestamp, until_timestamp, max_count], |row| {
                Ok((
                    row.get::<usize, String>(0).unwrap(),
                    row.get::<usize, usize>(1).unwrap(),
                ))
            })?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let (cat_ids, count) = row.unwrap();
                let cat_ids = cat_ids
                    .split(';')
                    .map(|val| val.parse::<usize>().unwrap())
                    .collect();
                (cat_ids, count)
            })
            .collect())
    }

    fn calculate_data_counts(
        &self,
        from: &NaiveDate,
        to: &NaiveDate,
        category_ids: &[usize],
    ) -> Result<Vec<usize>> {
        let mut result = vec![];
        for &cat_id in category_ids {
            let mut statement = self.connection.prepare(
                "SELECT COUNT(*) FROM EntryToCategories WHERE category_id=?1 AND date<=?2 AND date>=?3",
            )?;
            let from_timestamp = date_to_timestamp(from);
            let to_timestamp = date_to_timestamp(to);
            let count = statement
                .query_row(params![cat_id, from_timestamp, to_timestamp], |row| {
                    row.get(0)
                })?;
            result.push(count);
        }
        Ok(result)
    }

    fn update_data_internal(
        &mut self,
        new_items: &[(NaiveDate, Vec<usize>)],
    ) -> Result<SuccessfulUpdate> {
        let mut statement = self.connection.prepare("BEGIN")?;
        statement.execute([])?;
        let mut deleted_date_entries = 0;

        for (date, new_category_ids) in new_items {
            // Remove entry in DateEntry if exists
            let mut statement = self
                .connection
                .prepare("DELETE FROM DateEntry WHERE date=?1")?;
            let date_timestamp = date_to_timestamp(date);
            deleted_date_entries += statement.execute([date_timestamp])?;

            // Add entry in DateEntry
            let now = chrono::Local::now().timestamp();
            let mut statement = self
                .connection
                .prepare("INSERT INTO DateEntry (date, created_at) VALUES (?1, ?2)")?;
            statement.execute(params![date_timestamp, now])?;

            // Add new associations
            let mut statement = self
                .connection
                .prepare("INSERT INTO EntryToCategories (date, category_id) VALUES (?1, ?2)")?;
            for id in new_category_ids {
                statement.execute(params![date_timestamp, id])?;
            }
        }

        let mut statement = self.connection.prepare("COMMIT")?;
        statement.execute([])?;

        if deleted_date_entries == 0 {
            Ok(SuccessfulUpdate::AddedNew)
        } else {
            Ok(SuccessfulUpdate::ReplacedExisting)
        }
    }

    fn get_db_version(&self) -> Result<usize> {
        if let Ok(mut statement) = self
            .connection
            .prepare("SELECT info_value FROM Info WHERE info_name=\"version\"")
        {
            let version: Result<String, rusqlite::Error> =
                statement.query_row([], |row| row.get(0));
            version
                .map(|str| Ok(str.parse().unwrap_or(0)))
                .unwrap_or(Ok(0))
        } else {
            Ok(0)
        }
    }

    fn update_db_to_v1(&self) -> Result<()> {
        println!("- Updating SQLite datafile to version 1...");
        self.connection.execute_batch(
            "BEGIN;
            DROP TABLE IF EXISTS Info;
            CREATE TABLE Info(
                info_id INTEGER PRIMARY KEY AUTOINCREMENT,
                info_name TEXT UNIQUE NOT NULL,
                info_value TEXT NOT NULL
            );
            ALTER TABLE Category ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;
            COMMIT;",
        )?;
        insert_version_to_db(&self.connection)?;
        println!("- Success");
        Ok(())
    }

    fn update_db(&self) -> Result<()> {
        self.update_db_to_v1()?;
        Ok(())
    }

    fn get_visible_category_ids(&self) -> Result<Vec<usize>> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id FROM Category WHERE hidden=0 ORDER BY category_id")?;
        let rows = statement.query_map([], |row| row.get(0))?;

        // Ordered list of all category IDs in the database
        let mut category_ids: Vec<usize> = vec![];
        for id in rows {
            category_ids.push(id?);
        }
        Ok(category_ids)
    }
}

/// Tries to read data file to memory.
pub fn open_datafile(path: &Path) -> Result<DiaryDataSqlite> {
    let connection = Connection::open(path).context("Could not open SQLite database")?;
    {
        let mut backup_ext = OsString::from(path.extension().unwrap_or_default());
        backup_ext.push(".bak");
        let backup_path = path.with_extension(backup_ext);
        let mut backup_connection =
            Connection::open(backup_path).context("Could not open SQLite database for backup")?;
        let backup = backup::Backup::new(&connection, &mut backup_connection)
            .context("Could not initiate database backup")?;
        backup
            .run_to_completion(10, std::time::Duration::default(), None)
            .context("Could not perform backup")?;
    }
    open_sqlite_database(connection)
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
                *from_date - chrono::Duration::try_days(start as i64).unwrap(),
                *from_date - chrono::Duration::try_days(end as i64).unwrap(),
            )
        })
        .collect()
}

/// Create a new database on the prescribed path, using the prescribed headers.
pub fn create_new_datafile(path: &Path, headers: &[String]) -> Result<()> {
    create_new_sqlite(path, headers)?;
    Ok(())
}

#[test]
fn test_get_date_ranges() {
    let result = get_date_ranges(&NaiveDate::from_ymd_opt(2000, 5, 30).unwrap(), 5, 3);
    assert_eq!(
        vec![
            (
                NaiveDate::from_ymd_opt(2000, 5, 30).unwrap(),
                NaiveDate::from_ymd_opt(2000, 5, 26).unwrap()
            ),
            (
                NaiveDate::from_ymd_opt(2000, 5, 25).unwrap(),
                NaiveDate::from_ymd_opt(2000, 5, 21).unwrap()
            ),
            (
                NaiveDate::from_ymd_opt(2000, 5, 20).unwrap(),
                NaiveDate::from_ymd_opt(2000, 5, 16).unwrap()
            ),
        ],
        result
    );
}
