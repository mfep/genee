//! Handling SQLite habit databases.
use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime};
use std::path::Path;

use super::DiaryDataConnection;
use rusqlite::{params, Connection};

struct DiaryDataSqlite {
    connection: Connection,
}

pub fn create_new_sqlite(path: &Path, headers: &[String]) -> Result<()> {
    let conn = Connection::open(path).context("Could not open/create SQLite database")?;
    conn.execute_batch(
        "BEGIN;
        DROP TABLE IF EXISTS Category;
        CREATE TABLE Category(
            category_id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            created_at INTEGER NOT NULL
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
    let now = chrono::Local::now().timestamp();
    for header in headers {
        conn.execute(
            "INSERT INTO Category (name, created_at) VALUES (?1, ?2)",
            params![header, now],
        )?;
    }
    Ok(())
}

pub fn open_sqlite_datafile(path: &Path) -> Result<Box<dyn DiaryDataConnection>> {
    let data = DiaryDataSqlite {
        connection: Connection::open(path).context("Could not open SQLite database")?,
    };
    Ok(Box::new(data))
}

impl DiaryDataConnection for DiaryDataSqlite {
    fn calculate_data_counts_per_iter(
        &self,
        date_ranges: &[(chrono::NaiveDate, chrono::NaiveDate)],
    ) -> Result<Vec<Vec<usize>>> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id FROM Category ORDER BY category_id")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        let mut cat_ids = vec![];
        for cat_id in rows {
            cat_ids.push(cat_id?);
        }
        let mut result = vec![];
        for (from, to) in date_ranges {
            result.push(self.calculate_data_counts(from, to, &cat_ids)?);
        }
        Ok(result)
    }

    fn update_data(
        &mut self,
        date: &chrono::NaiveDate,
        new_row: &[bool],
    ) -> Result<super::SuccessfulUpdate> {
        self.update_data_internal(&[(*date, new_row.to_vec())])
    }

    fn update_data_batch(&mut self, new_items: &[(NaiveDate, Vec<bool>)]) -> Result<()> {
        self.update_data_internal(new_items)?;
        Ok(())
    }

    fn get_missing_dates(
        &self,
        from: &Option<chrono::NaiveDate>,
        until: &chrono::NaiveDate,
    ) -> Result<Vec<chrono::NaiveDate>> {
        if self.is_empty()? {
            return Ok(vec![]);
        }

        // If no from, select the earliest date in the database
        let from = match from {
            Some(date) => *date,
            None => {
                let mut statement = self.connection.prepare("SELECT MIN(date) FROM DateEntry")?;
                let min_date = statement.query_row([], |row| row.get(0))?;
                NaiveDateTime::from_timestamp_opt(min_date, 0)
                    .unwrap()
                    .date()
            }
        };

        let mut statement = self
            .connection
            .prepare("SELECT date FROM DateEntry WHERE date>=?1 AND date<=?2")?;
        let from_timestamp = from.and_time(chrono::NaiveTime::default()).timestamp();
        let until_timestamp = until.and_time(chrono::NaiveTime::default()).timestamp();
        let rows = statement.query_map([from_timestamp, until_timestamp], |row| row.get(0))?;
        let mut missing_dates = vec![];

        let mut current_date = from;
        for date_val in rows {
            let next_present_day = NaiveDateTime::from_timestamp_opt(date_val?, 0)
                .unwrap()
                .date();
            while current_date <= *until {
                let last_date = current_date;
                current_date += chrono::Duration::days(1);
                if next_present_day == last_date {
                    break;
                }
                missing_dates.push(last_date);
            }
        }
        while current_date <= *until {
            missing_dates.push(current_date);
            current_date += chrono::Duration::days(1);
        }

        Ok(missing_dates)
    }

    fn get_header(&self) -> Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT name FROM Category ORDER BY category_id")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        let mut ret = vec![];

        for name in rows {
            ret.push(name?);
        }
        Ok(ret)
    }

    fn get_row(&self, date: &chrono::NaiveDate) -> Result<Option<Vec<bool>>> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id FROM Category ORDER BY category_id")?;
        let rows = statement.query_map([], |row| row.get(0))?;

        // Ordered list of all category IDs in the database
        let mut cat_ids: Vec<usize> = vec![];
        for id in rows {
            cat_ids.push(id?);
        }

        // Get if the date exists in the database
        let mut statement = self
            .connection
            .prepare("SELECT COUNT(*) FROM DateEntry WHERE date=?1")?;
        let date_timestamp = date.and_time(chrono::NaiveTime::default()).timestamp();
        let date_count: usize = statement.query_row([date_timestamp], |row| row.get(0))?;
        let day_exists = date_count > 0;
        if !day_exists {
            return Ok(None);
        }

        // Get categories for the specified date
        let mut statement = self.connection.prepare(
            "SELECT category_id FROM EntryToCategories WHERE date=(?1) ORDER BY category_id",
        )?;
        let rows = statement.query_map([date_timestamp], |row| row.get(0))?;
        let mut cat_ids_for_date: Vec<usize> = vec![];
        for id in rows {
            cat_ids_for_date.push(id?);
        }

        let mut res = vec![];
        for cat_id in &cat_ids {
            res.push(cat_ids_for_date.contains(cat_id));
        }
        Ok(Some(res))
    }

    fn is_empty(&self) -> Result<bool> {
        let mut statement = self.connection.prepare("SELECT COUNT(*) FROM DateEntry")?;
        let count: usize = statement.query_row([], |row| row.get(0))?;
        Ok(count == 0)
    }
}

impl DiaryDataSqlite {
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
            let from_timestamp = from.and_time(chrono::NaiveTime::default()).timestamp();
            let to_timestamp = to.and_time(chrono::NaiveTime::default()).timestamp();
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
        new_items: &[(NaiveDate, Vec<bool>)],
    ) -> Result<super::SuccessfulUpdate> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id FROM Category ORDER BY category_id")?;
        let rows = statement.query_map([], |row| row.get(0))?;

        // Ordered list of all category IDs in the database
        let mut cat_ids: Vec<usize> = vec![];
        for id in rows {
            cat_ids.push(id?);
        }
        let mut statement = self.connection.prepare("BEGIN")?;
        statement.execute([])?;
        let mut deleted_date_entries = 0;

        for (date, new_row) in new_items {
            // The IDs of the inserted categories for the date
            let mut updated_cat_ids = vec![];
            for (&id, &marked) in cat_ids.iter().zip(new_row.iter()) {
                if marked {
                    updated_cat_ids.push(id);
                }
            }

            // Remove entry in DateEntry if exists
            let mut statement = self
                .connection
                .prepare("DELETE FROM DateEntry WHERE date=?1")?;
            let date_timestamp = date.and_time(chrono::NaiveTime::default()).timestamp();
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
            for id in updated_cat_ids {
                statement.execute(params![date_timestamp, id])?;
            }
        }

        let mut statement = self.connection.prepare("COMMIT")?;
        statement.execute([])?;

        if deleted_date_entries == 0 {
            Ok(super::SuccessfulUpdate::AddedNew)
        } else {
            Ok(super::SuccessfulUpdate::ReplacedExisting)
        }
    }
}

#[test]
fn test_sqlite() {
    create_new_sqlite(
        Path::new("test.db"),
        &[String::from("AA"), String::from("BBB"), String::from("CCA")],
    )
    .unwrap();
    let mut datafile = open_sqlite_datafile(Path::new("test.db")).unwrap();
    datafile
        .update_data(
            &chrono::NaiveDate::from_ymd_opt(2023, 2, 4).unwrap(),
            &[false, true, false],
        )
        .unwrap();
    datafile
        .update_data(
            &chrono::NaiveDate::from_ymd_opt(2023, 3, 3).unwrap(),
            &[false, true, false],
        )
        .unwrap();
    datafile
        .update_data(
            &chrono::NaiveDate::from_ymd_opt(2023, 2, 7).unwrap(),
            &[false, false, true],
        )
        .unwrap();
    let missing_dates = datafile
        .get_missing_dates(
            &None,
            &chrono::NaiveDate::from_ymd_opt(2023, 2, 10).unwrap(),
        )
        .unwrap();
    assert_eq!(
        missing_dates,
        vec![
            NaiveDate::from_ymd_opt(2023, 2, 5).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 6).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 8).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 9).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 10).unwrap(),
        ]
    );
    let data_counts = datafile
        .calculate_data_counts_per_iter(&vec![(
            NaiveDate::from_ymd_opt(2023, 3, 3).unwrap(),
            NaiveDate::from_ymd_opt(2023, 2, 3).unwrap(),
        )])
        .unwrap();
    assert_eq!(data_counts, vec![vec![0, 2, 1]]);
}
