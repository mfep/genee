use std::io::stdout;

use crate::CliOptions;
use anyhow::Result;
use chrono::NaiveDate;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use genee::datafile::{self, DiaryDataConnection};
use ratatui::{prelude::*, widgets::*};

pub fn run_app(opts: &CliOptions) -> Result<()> {
    let mut app = UiApp::new(opts)?;

    crossterm::terminal::enable_raw_mode()?;
    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            render_habit_table(frame, &mut app);
        })?;
        should_quit = handle_events(&mut app)?;
    }

    crossterm::terminal::disable_raw_mode()?;
    stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

const DEFAULT_STARTING_HABIT_ROWS: usize = 100;

struct HabitEditState {
    date: NaiveDate,
    row_index: usize,
    initial_habit_vec: Vec<bool>,
    habit_vec: Vec<bool>,
}

enum AppState {
    Browsing,
    Editing(HabitEditState),
}

struct UiApp {
    datafile: Box<dyn DiaryDataConnection>,
    header: Vec<(String, usize)>,
    habit_table_state: TableState,
    habit_rows: Vec<(NaiveDate, Option<Vec<bool>>)>,
    start_date: NaiveDate,
    state: AppState,
    edit_col_idx: usize,
}

impl UiApp {
    fn new(opts: &CliOptions) -> Result<Self> {
        let datafile = datafile::open_datafile(opts.datafile.as_ref().unwrap())?;
        let start_date = chrono::Local::now().date_naive();
        let mut habit_table_state = TableState::default();
        habit_table_state.select(Some(0));

        let mut app = UiApp {
            header: datafile.get_header()?,
            habit_table_state,
            habit_rows: vec![],
            datafile,
            start_date,
            state: AppState::Browsing,
            edit_col_idx: 0,
        };
        app.load_habit_row_batch(&start_date)?;
        Ok(app)
    }

    fn load_habit_row_batch(&mut self, batch_start_date: &NaiveDate) -> Result<()> {
        let mut date = *batch_start_date;
        for _i in 0..DEFAULT_STARTING_HABIT_ROWS {
            let habit_data = self.datafile.get_row(&date)?;
            self.habit_rows.push((
                date,
                habit_data.map(|cat_ids| decode_habit_vector(&self.header, &cat_ids)),
            ));
            date -= chrono::Duration::days(1);
        }
        Ok(())
    }

    fn ensure_habit_row_index(&mut self, index: usize) -> Result<()> {
        if index >= self.habit_rows.len() {
            self.load_habit_row_batch(&(self.start_date - chrono::Duration::days(index as i64)))?;
        }
        Ok(())
    }
}

fn render_habit_table(frame: &mut Frame, app: &mut UiApp) {
    let widths: Vec<Constraint> = (0..app.header.len() + 1)
        .map(|i| {
            if i == 0 {
                Constraint::Min(12)
            } else {
                Constraint::Min(3)
            }
        })
        .collect();

    let rows = get_daily_habit_rows(app);

    let table = Table::new(rows, widths)
        .header(get_table_header(&app.header))
        .block(Block::new().borders(Borders::ALL));
    frame.render_stateful_widget(table, frame.size(), &mut app.habit_table_state);
}

fn handle_events(app: &mut UiApp) -> Result<bool> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match &mut app.state {
                AppState::Browsing => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Up {
                        app.habit_table_state
                            .select(app.habit_table_state.selected().map(|idx| {
                                if idx != 0 {
                                    idx - 1
                                } else {
                                    0
                                }
                            }));
                    }
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Down {
                        if let Some(current_idx) = app.habit_table_state.selected() {
                            let new_idx = current_idx + 1;
                            app.ensure_habit_row_index(new_idx)?;
                            app.habit_table_state.select(Some(new_idx));
                        }
                    }
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                        let row_index = app.habit_table_state.selected().unwrap();
                        let habit_vec = app.habit_rows[row_index]
                            .1
                            .clone()
                            .unwrap_or_else(|| vec![false; app.header.len()]);
                        app.state = AppState::Editing(HabitEditState {
                            date: app.habit_rows[row_index].0,
                            row_index,
                            initial_habit_vec: habit_vec.clone(),
                            habit_vec,
                        });
                    }
                }
                AppState::Editing(edit_state) => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                        if edit_state.habit_vec != edit_state.initial_habit_vec {
                            let row_idx = (app.start_date - edit_state.date).num_days();
                            app.habit_rows[row_idx as usize].1 = Some(edit_state.habit_vec.clone());
                            app.datafile.update_data(
                                &edit_state.date,
                                &encode_habit_vector(&app.header, &edit_state.habit_vec),
                            )?;
                        }
                        app.state = AppState::Browsing;
                    } else if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Left
                        && app.edit_col_idx > 0
                    {
                        app.edit_col_idx -= 1;
                    } else if key.kind == KeyEventKind::Press
                        && key.code == KeyCode::Right
                        && app.edit_col_idx < app.header.len() - 1
                    {
                        app.edit_col_idx += 1;
                    } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char(' ') {
                        let entry = &mut edit_state.habit_vec[app.edit_col_idx];
                        *entry = !*entry;
                    }
                }
            }
        }
    }
    Ok(false)
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

fn get_daily_habit_rows<'a>(app: &UiApp) -> Vec<Row<'a>> {
    let categories = &app.header;
    let mut rows = vec![];
    for (row_idx, data_row) in app.habit_rows.iter().enumerate() {
        let mut cells = vec![Cell::new(data_row.0.to_string())];
        let (habit_vector, edited_col_idx) = match &app.state {
            AppState::Browsing => (data_row.1.as_ref(), None),
            AppState::Editing(edit_state) => {
                if row_idx == edit_state.row_index {
                    (Some(&edit_state.habit_vec), Some(app.edit_col_idx))
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
        if app
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
