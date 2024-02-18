use anyhow::Result;
use chrono::NaiveDate;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use genee::datafile::DiaryDataConnection;
use ratatui::{prelude::*, widgets::*};

const DEFAULT_STARTING_HABIT_ROWS: usize = 100;

struct EditState {
    date: NaiveDate,
    row_index: usize,
    initial_habit_vec: Vec<bool>,
    habit_vec: Vec<bool>,
}

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
}

pub struct HabitDayListWidgetContext<'a> {
    datafile: &'a mut dyn DiaryDataConnection,
}

impl<'a> HabitDayListWidgetContext<'a> {
    pub fn new(datafile: &'a mut dyn DiaryDataConnection) -> HabitDayListWidgetContext<'a> {
        HabitDayListWidgetContext { datafile }
    }
}

impl HabitDayListWidget {
    pub fn new(context: HabitDayListWidgetContext<'_>) -> Result<Self> {
        let start_date = chrono::Local::now().date_naive();
        let mut habit_table_state = TableState::default();
        habit_table_state.select(Some(0));

        let mut widget = HabitDayListWidget {
            header: context.datafile.get_header()?,
            habit_table_state,
            habit_rows: vec![],
            start_date,
            state: WidgetState::Browsing,
            edit_col_idx: 0,
        };
        widget.load_habit_row_batch(context, &start_date)?;
        Ok(widget)
    }

    fn load_habit_row_batch(
        &mut self,
        context: HabitDayListWidgetContext<'_>,
        batch_start_date: &NaiveDate,
    ) -> Result<()> {
        let from = *batch_start_date - chrono::Duration::days(DEFAULT_STARTING_HABIT_ROWS as i64);
        let new_rows = context.datafile.get_rows(&from, batch_start_date)?;

        let mut date = *batch_start_date;
        for row in new_rows {
            self.habit_rows.push((
                date,
                row.map(|cat_ids| decode_habit_vector(&self.header, &cat_ids)),
            ));
            date -= chrono::Duration::days(1);
        }
        Ok(())
    }

    fn ensure_habit_row_index(
        &mut self,
        context: HabitDayListWidgetContext<'_>,
        index: usize,
    ) -> Result<()> {
        if index >= self.habit_rows.len() {
            self.load_habit_row_batch(
                context,
                &(self.start_date - chrono::Duration::days(index as i64)),
            )?;
        }
        Ok(())
    }

    pub fn handle_events(
        &mut self,
        context: HabitDayListWidgetContext<'_>,
        event: &Event,
    ) -> Result<()> {
        if let Event::Key(key) = event {
            match &mut self.state {
                WidgetState::Browsing => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Up {
                        self.habit_table_state
                            .select(self.habit_table_state.selected().map(|idx| {
                                if idx != 0 {
                                    idx - 1
                                } else {
                                    0
                                }
                            }));
                    }
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Down {
                        if let Some(current_idx) = self.habit_table_state.selected() {
                            let new_idx = current_idx + 1;
                            self.ensure_habit_row_index(context, new_idx)?;
                            self.habit_table_state.select(Some(new_idx));
                        }
                    }
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
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
                    }
                }
                WidgetState::Editing(edit_state) => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                        if edit_state.habit_vec != edit_state.initial_habit_vec {
                            let row_idx = (self.start_date - edit_state.date).num_days();
                            self.habit_rows[row_idx as usize].1 =
                                Some(edit_state.habit_vec.clone());
                            context.datafile.update_data(
                                &edit_state.date,
                                &encode_habit_vector(&self.header, &edit_state.habit_vec),
                            )?;
                        }
                        self.state = WidgetState::Browsing;
                    } else if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Left
                        && self.edit_col_idx > 0
                    {
                        self.edit_col_idx -= 1;
                    } else if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Right
                        && self.edit_col_idx < self.header.len() - 1
                    {
                        self.edit_col_idx += 1;
                    } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char(' ') {
                        let entry = &mut edit_state.habit_vec[self.edit_col_idx];
                        *entry = !*entry;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widths: Vec<Constraint> = (0..self.header.len() + 1)
            .map(|i| {
                if i == 0 {
                    Constraint::Min(12)
                } else {
                    Constraint::Min(3)
                }
            })
            .collect();

        let rows = get_daily_habit_rows(self);

        let table = Table::new(rows, widths)
            .header(get_table_header(&self.header))
            .block(Block::new().borders(Borders::ALL));
        frame.render_stateful_widget(table, area, &mut self.habit_table_state);
    }
}

fn get_table_header<'a>(header: &[(String, usize)]) -> Row<'a> {
    let mut cells = vec![Cell::new("Date")];
    for (name, _idx) in header {
        cells.push(Cell::new(name.clone()));
    }
    Row::new(cells).add_modifier(Modifier::BOLD)
}

fn decode_habit_vector(categories: &[(String, usize)], ids: &[usize]) -> Vec<bool> {
    let mut v = vec![];
    for (_, cat_id) in categories {
        v.push(ids.contains(cat_id));
    }
    v
}

fn encode_habit_vector(categories: &[(String, usize)], entries: &[bool]) -> Vec<usize> {
    assert_eq!(categories.len(), entries.len());
    let mut entry_ids = vec![];
    for (val, (_name, cat_id)) in entries.iter().zip(categories.iter()) {
        if *val {
            entry_ids.push(*cat_id);
        }
    }
    entry_ids
}

fn get_daily_habit_rows<'a>(widget: &HabitDayListWidget) -> Vec<Row<'a>> {
    let categories = &widget.header;
    let mut rows = vec![];
    for (row_idx, data_row) in widget.habit_rows.iter().enumerate() {
        let mut cells = vec![Cell::new(data_row.0.to_string())];
        let (habit_vector, edited_col_idx) = match &widget.state {
            WidgetState::Browsing => (data_row.1.as_ref(), None),
            WidgetState::Editing(edit_state) => {
                if row_idx == edit_state.row_index {
                    (Some(&edit_state.habit_vec), Some(widget.edit_col_idx))
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
        if widget
            .habit_table_state
            .selected()
            .map_or(false, |selected_idx| selected_idx == row_idx)
        {
            rows.push(row.bg(Color::DarkGray));
        } else {
            rows.push(row);
        }
    }
    rows
}
