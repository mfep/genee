//! Handling SQLite habit databases.
use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, NaiveDateTime};
use std::{ffi::OsString, path::Path};

use super::DiaryDataConnection;
use rusqlite::{backup, params, Connection};

struct DiaryDataSqlite {
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

pub fn create_new_sqlite(path: &Path, headers: &[String]) -> Result<()> {
    let conn = Connection::open(path).context("Could not open/create SQLite database")?;
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
    insert_version_to_db(&conn)?;
    let now = chrono::Local::now().timestamp();
    for header in headers {
        conn.execute(
            "INSERT INTO Category (name, created_at, hidden) VALUES (?1, ?2, 0)",
            params![header, now],
        )?;
    }
    Ok(())
}

pub fn open_sqlite_datafile(path: &Path) -> Result<Box<dyn DiaryDataConnection>> {
    let data = DiaryDataSqlite {
        connection: Connection::open(path).context("Could not open SQLite database")?,
    };
    {
        let mut backup_ext = OsString::from(path.extension().unwrap_or_default());
        backup_ext.push(".bak");
        let backup_path = path.with_extension(backup_ext);
        let mut backup_connection =
            Connection::open(backup_path).context("Could not open SQLite database for backup")?;
        let backup = backup::Backup::new(&data.connection, &mut backup_connection)
            .context("Could not initiate database backup")?;
        backup
            .run_to_completion(10, std::time::Duration::default(), None)
            .context("Could not perform backup")?;
    }
    let db_version = data.get_db_version()?;
    if db_version < CURRENT_DB_VERSION {
        println!(
            "Detected an SQLite datafile of version {}. Commencing update...",
            db_version
        );
        data.update_db()?;
    }
    Ok(Box::new(data))
}

impl DiaryDataConnection for DiaryDataSqlite {
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

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
            .prepare("SELECT name FROM Category WHERE hidden=0 ORDER BY category_id")?;
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
            .prepare("SELECT category_id FROM Category WHERE hidden=0 ORDER BY category_id")?;
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

    fn get_date_range(&self) -> Result<(NaiveDate, NaiveDate)> {
        if self.is_empty()? {
            bail!("Cannot get date range, datafile is empty")
        }

        let mut statement = self
            .connection
            .prepare("SELECT MIN(date), MAX(date) FROM DateEntry")?;
        let mut rows = statement.query([])?;
        let row = rows.next()?.unwrap();
        let min_date = NaiveDateTime::from_timestamp_opt(row.get(0)?, 0)
            .unwrap()
            .date();
        let max_date = NaiveDateTime::from_timestamp_opt(row.get(1)?, 0)
            .unwrap()
            .date();

        Ok((min_date, max_date))
    }

    fn add_category(&self, name: &str) -> Result<super::AddCategoryResult> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id, hidden FROM Category WHERE name=(?1)")?;
        let mut rows = statement.query(params![name])?;

        if let Some(row) = rows.next()? {
            let category_id: usize = row.get(0)?;
            let hidden = 0usize != row.get(1)?;

            if hidden {
                let mut statement = self
                    .connection
                    .prepare("UPDATE Category SET hidden=0 WHERE category_id=(?1)")?;
                statement.execute(params![category_id])?;
                Ok(super::AddCategoryResult::Unhide)
            } else {
                Ok(super::AddCategoryResult::AlreadyPresent)
            }
        } else {
            let mut statement = self
                .connection
                .prepare("INSERT INTO Category (name, created_at, hidden) VALUES (?1, ?2, 0)")?;
            let now = chrono::Local::now().timestamp();
            statement.execute(params![name, now])?;
            Ok(super::AddCategoryResult::AddedNew)
        }
    }

    fn hide_category(&self, name: &str) -> Result<super::HideCategoryResult> {
        let mut statement = self
            .connection
            .prepare("SELECT category_id, hidden FROM Category WHERE name=(?1)")?;
        let mut rows = statement.query(params![name])?;
        if let Some(row) = rows.next()? {
            let category_id: usize = row.get(0)?;
            let hidden = 0usize != row.get(1)?;
            if hidden {
                Ok(super::HideCategoryResult::AlreadyHidden)
            } else {
                let mut statement = self
                    .connection
                    .prepare("UPDATE Category SET hidden=1 WHERE category_id=(?1)")?;
                statement.execute(params![category_id])?;
                Ok(super::HideCategoryResult::Hidden)
            }
        } else {
            Ok(super::HideCategoryResult::NonExistingCategory)
        }
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
            ALTER TABLE Category ADD COLUMN hidden INTEGER NOT NULL;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_database_version() {
        create_new_sqlite(
            Path::new("test.db"),
            &[String::from("AA"), String::from("BBB"), String::from("CCA")],
        )
        .unwrap();
        let datafile = open_sqlite_datafile(Path::new("test.db")).unwrap();

        assert_eq!(
            CURRENT_DB_VERSION,
            datafile
                .into_any()
                .downcast::<DiaryDataSqlite>()
                .unwrap()
                .get_db_version()
                .unwrap()
        );
    }

    #[test]
    fn database_update() {
        {
            let conn = Connection::open("test.db").unwrap();
            conn.execute_batch(
                "BEGIN;
                DROP TABLE IF EXISTS Info;
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
            )
            .unwrap();
            conn.close().unwrap();
        }
        let datafile = open_sqlite_datafile(Path::new("test.db")).unwrap();

        assert_eq!(
            CURRENT_DB_VERSION,
            datafile
                .into_any()
                .downcast::<DiaryDataSqlite>()
                .unwrap()
                .get_db_version()
                .unwrap()
        );
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

        let (min_date, max_date) = datafile.get_date_range().unwrap();
        assert_eq!(
            min_date,
            chrono::NaiveDate::from_ymd_opt(2023, 2, 4).unwrap()
        );
        assert_eq!(
            max_date,
            chrono::NaiveDate::from_ymd_opt(2023, 3, 3).unwrap()
        );
    }

    #[test]
    fn add_category() {
        use crate::datafile::AddCategoryResult;

        create_new_sqlite(
            Path::new("test.db"),
            &[String::from("AA"), String::from("BBB"), String::from("CCA")],
        )
        .unwrap();
        let datafile = open_sqlite_datafile(Path::new("test.db")).unwrap();
        let result = datafile.add_category("BBB").unwrap();
        assert_eq!(AddCategoryResult::AlreadyPresent, result);
        let result = datafile.add_category("DDD").unwrap();
        assert_eq!(AddCategoryResult::AddedNew, result);

        let header = datafile.get_header().unwrap();
        assert_eq!(vec!["AA", "BBB", "CCA", "DDD"], header);
    }

    #[test]
    fn hide_category() {
        use crate::datafile::{AddCategoryResult, HideCategoryResult};

        create_new_sqlite(
            Path::new("test.db"),
            &[String::from("AA"), String::from("BBB"), String::from("CCA")],
        )
        .unwrap();
        let datafile = open_sqlite_datafile(Path::new("test.db")).unwrap();

        let result = datafile.hide_category("DDD").unwrap();
        assert_eq!(HideCategoryResult::NonExistingCategory, result);

        let result = datafile.hide_category("AA").unwrap();
        assert_eq!(HideCategoryResult::Hidden, result);

        let header = datafile.get_header().unwrap();
        assert_eq!(vec!["BBB", "CCA"], header);

        let result = datafile.hide_category("AA").unwrap();
        assert_eq!(HideCategoryResult::AlreadyHidden, result);

        let header = datafile.get_header().unwrap();
        assert_eq!(vec!["BBB", "CCA"], header);

        let result = datafile.add_category("AA").unwrap();
        assert_eq!(AddCategoryResult::Unhide, result);

        let header = datafile.get_header().unwrap();
        assert_eq!(vec!["AA", "BBB", "CCA"], header);
    }
}
