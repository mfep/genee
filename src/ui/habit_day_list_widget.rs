use super::{Scale, table_utils};
use anyhow::Result;
use chrono::NaiveDate;
use genee::datafile::DiaryDataConnection;
use ratatui::{prelude::*, widgets::*};

const DEFAULT_STARTING_HABIT_ROWS: usize = 100;

#[derive(PartialEq)]
struct EditState {
    date: NaiveDate,
    row_index: usize,
    initial_habit_vec: Vec<bool>,
    habit_vec: Vec<bool>,
}

#[derive(PartialEq)]
enum WidgetState {
    Browsing,
    Editing(EditState),
}

pub struct HabitDayListWidget {
    header: Vec<(String, usize)>,
    habit_table_state: TableState,
    habit_rows: Vec<(NaiveDate, Option<Vec<bool>>)>,
    start_date: NaiveDate,
    state: WidgetState,
    edit_col_idx: usize,
    scale: Scale,
}

pub enum HabitDayListWidgetInput {
    StepEarlier,
    StepLater,
    StrideEarlier,
    StrideLater,
    NavigateColumn(isize),
    SwitchMode,
    SwitchValue,
}

impl HabitDayListWidget {
    pub fn new(datafile: &dyn DiaryDataConnection, start_date: NaiveDate) -> Result<Self> {
        let mut habit_table_state = TableState::default();
        habit_table_state.select(Some(0));

        let mut widget = HabitDayListWidget {
            header: datafile.get_header()?,
            habit_table_state,
            habit_rows: vec![],
            start_date,
            state: WidgetState::Browsing,
            edit_col_idx: 0,
            scale: Scale::Monthly,
        };
        widget.load_habit_row_batch(datafile, &start_date)?;
        Ok(widget)
    }

    pub fn update(
        &mut self,
        datafile: &mut dyn DiaryDataConnection,
        input: HabitDayListWidgetInput,
    ) -> Result<()> {
        match input {
            HabitDayListWidgetInput::StepEarlier => {
                self.navigate_date(datafile, 1)?;
            }
            HabitDayListWidgetInput::StepLater => {
                self.navigate_date(datafile, -1)?;
            }
            HabitDayListWidgetInput::StrideEarlier => {
                self.navigate_date(datafile, self.scale.value() as isize)?;
            }
            HabitDayListWidgetInput::StrideLater => {
                self.navigate_date(datafile, -(self.scale.value() as isize))?;
            }
            HabitDayListWidgetInput::NavigateColumn(offset) => {
                if let WidgetState::Editing(_) = &self.state {
                    let new_val = (self.edit_col_idx as isize) + offset;
                    if new_val >= 0 && new_val < self.header.len() as isize {
                        self.edit_col_idx = new_val as usize;
                    }
                } else if offset > 0 {
                    self.scale = self.scale.larger();
                } else {
                    self.scale = self.scale.smaller();
                }
            }
            HabitDayListWidgetInput::SwitchMode => {
                if let WidgetState::Browsing = &self.state {
                    let row_index = self.habit_table_state.selected().unwrap();
                    let habit_vec = self.habit_rows[row_index]
                        .1
                        .clone()
                        .unwrap_or_else(|| vec![false; self.header.len()]);
                    self.state = WidgetState::Editing(EditState {
                        date: self.habit_rows[row_index].0,
                        row_index,
                        initial_habit_vec: habit_vec.clone(),
                        habit_vec,
                    });
                } else if let WidgetState::Editing(edit_state) = &self.state {
                    if edit_state.habit_vec != edit_state.initial_habit_vec {
                        let row_idx = (self.start_date - edit_state.date).num_days();
                        self.habit_rows[row_idx as usize].1 = Some(edit_state.habit_vec.clone());
                        datafile.update_data(
                            &edit_state.date,
                            &table_utils::encode_habit_vector(&self.header, &edit_state.habit_vec),
                        )?;
                    }
                    self.state = WidgetState::Browsing;
                }
            }
            HabitDayListWidgetInput::SwitchValue => {
                if let WidgetState::Editing(edit_state) = &mut self.state {
                    let entry = &mut edit_state.habit_vec[self.edit_col_idx];
                    *entry = !*entry;
                }
            }
        }
        Ok(())
    }

    pub fn get_selected_date(&self) -> Option<NaiveDate> {
        self.habit_table_state
            .selected()
            .map(|idx| self.habit_rows[idx].0)
    }

    pub fn get_scale(&self) -> Scale {
        self.scale
    }

    fn navigate_date(&mut self, datafile: &dyn DiaryDataConnection, offset: isize) -> Result<()> {
        assert_ne!(offset, 0);
        if let WidgetState::Browsing = &self.state {
            let current_row_idx = self.habit_table_state.selected().unwrap_or_default() as isize;
            let new_row_idx = (current_row_idx - offset).max(0isize) as usize;
            self.ensure_habit_row_index(datafile, new_row_idx)?;
            self.habit_table_state.select(Some(new_row_idx));
        }
        Ok(())
    }

    fn load_habit_row_batch(
        &mut self,
        datafile: &dyn DiaryDataConnection,
        batch_start_date: &NaiveDate,
    ) -> Result<()> {
        let from = *batch_start_date
            - chrono::Duration::try_days(DEFAULT_STARTING_HABIT_ROWS as i64).unwrap();
        let new_rows = datafile.get_rows(&from, batch_start_date)?;

        let mut date = *batch_start_date;
        for row in new_rows {
            self.habit_rows.push((
                date,
                row.map(|cat_ids| table_utils::decode_habit_vector(&self.header, &cat_ids)),
            ));
            date -= chrono::Duration::try_days(1).unwrap();
        }
        Ok(())
    }

    fn ensure_habit_row_index(
        &mut self,
        datafile: &dyn DiaryDataConnection,
        index: usize,
    ) -> Result<()> {
        while index >= self.habit_rows.len() {
            self.load_habit_row_batch(
                datafile,
                &(self.start_date - chrono::Duration::try_days(index as i64).unwrap()),
            )?;
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths: Vec<Constraint> = (0..self.header.len() + 1)
            .map(|i| {
                if i == 0 {
                    Constraint::Max(12)
                } else {
                    Constraint::Max(3)
                }
            })
            .collect();

        let rows = self.get_daily_habit_rows();

        let table = Table::new(rows, widths)
            .header(table_utils::get_table_header(&self.header, "Date"))
            .block(
                Block::bordered()
                    .title(self.get_footer())
                    .title_position(block::Position::Bottom)
                    .title_top("Daily habit data"),
            );
        frame.render_stateful_widget(table, area, &mut self.habit_table_state);
    }

    fn get_footer(&self) -> String {
        match &self.state {
            WidgetState::Browsing => {
                format!(
                    "Step: <↑><↓> Stride ({}): <PgUp><PgDown> Edit: <RETURN> Change scale: <←><→> Exit: <Q>",
                    &self.scale
                )
            }
            WidgetState::Editing(_) => {
                String::from("Step: <←><→> Toggle: <SPACE> Apply: <RETURN> Exit: <Q>")
            }
        }
    }

    fn get_daily_habit_rows<'a>(&self) -> Vec<Row<'a>> {
        let categories = &self.header;
        let mut rows = vec![];
        for (row_idx, data_row) in self.habit_rows.iter().enumerate() {
            let mut cells = vec![Cell::new(data_row.0.to_string())];
            let (habit_vector, edited_col_idx) = match &self.state {
                WidgetState::Browsing => (data_row.1.as_ref(), None),
                WidgetState::Editing(edit_state) => {
                    if row_idx == edit_state.row_index {
                        (Some(&edit_state.habit_vec), Some(self.edit_col_idx))
                    } else {
                        (data_row.1.as_ref(), None)
                    }
                }
            };
            if let Some(habit_vector) = habit_vector {
                for (col_idx, val) in habit_vector.iter().enumerate() {
                    let span = if *val {
                        Span::from("✓")
                    } else {
                        Span::from(" ")
                    };
                    if edited_col_idx.map(|idx| idx == col_idx).unwrap_or(false) {
                        cells.push(Cell::new(span.bg(Color::LightGreen)));
                    } else {
                        cells.push(Cell::new(span));
                    }
                }
            } else {
                for _i in 0..categories.len() {
                    cells.push(Cell::new("?"));
                }
            }
            let row = Row::new(cells);
            if self.habit_table_state.selected() == Some(row_idx) {
                rows.push(row.bg(Color::DarkGray));
            } else {
                rows.push(row);
            }
        }
        rows
    }
}
