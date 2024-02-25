use super::table_utils;
use crate::datafile::DiaryDataConnection;
use anyhow::{Ok, Result};
use chrono::NaiveDate;
use ratatui::{prelude::*, widgets::*};

pub enum TopOccurrenceListWidgetInput {
    UpdateRange((NaiveDate, NaiveDate)),
}

pub struct TopOccurrenceListWidget {
    range_from: NaiveDate,
    range_until: NaiveDate,
    count: usize,
    header: Vec<(String, usize)>,
    data: Vec<(Vec<usize>, usize)>,
}

impl TopOccurrenceListWidget {
    pub fn new(
        datafile: &dyn DiaryDataConnection,
        range_from: NaiveDate,
        range_until: NaiveDate,
    ) -> Result<Self> {
        let count = 5usize;
        let header = datafile.get_header()?;
        let mut widget = TopOccurrenceListWidget {
            range_from,
            range_until,
            count,
            header,
            data: vec![],
        };
        widget.update_data(datafile)?;
        Ok(widget)
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths: Vec<Constraint> = (0..self.header.len() + 1)
            .map(|i| {
                if i == 0 {
                    Constraint::Min(5)
                } else {
                    Constraint::Min(3)
                }
            })
            .collect();
        let mut rows = vec![table_utils::get_table_header(&self.header, "Count")];
        for (ids, count) in &self.data {
            let mut cells = vec![Cell::new(format!("{:5}", count))];
            for habit_val in table_utils::decode_habit_vector(&self.header, ids) {
                cells.push(if habit_val {
                    Cell::from("✓")
                } else {
                    Cell::from(" ")
                });
            }
            rows.push(Row::new(cells));
        }
        let table = Table::new(rows, widths)
            .block(Block::default().borders(Borders::ALL).title(self.title()));
        frame.render_widget(table, area);
    }

    pub fn expected_height(&self) -> usize {
        self.count + 3
    }

    pub fn update(
        &mut self,
        datafile: &dyn DiaryDataConnection,
        input: TopOccurrenceListWidgetInput,
    ) -> Result<()> {
        match input {
            TopOccurrenceListWidgetInput::UpdateRange((from, until)) => {
                self.range_from = from;
                self.range_until = until;
                self.update_data(datafile)?;
            }
        }
        Ok(())
    }

    fn update_data(&mut self, datafile: &dyn DiaryDataConnection) -> Result<()> {
        self.data = datafile.get_most_frequent_daily_data(
            &Some(self.range_from),
            &self.range_until,
            Some(self.count),
        )?;
        Ok(())
    }

    fn title(&self) -> String {
        format!(
            "Most occuring daily habits from {} until {}",
            self.range_from, self.range_until
        )
    }
}
