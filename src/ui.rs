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

struct UiApp {
    datafile: Box<dyn DiaryDataConnection>,
    header: Vec<(String, usize)>,
    habit_table_state: TableState,
    habit_rows: Vec<(NaiveDate, Option<Vec<bool>>)>,
    start_date: NaiveDate,
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
                habit_data.map(|cat_ids| get_habit_vector(&self.header, &cat_ids)),
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
        .block(Block::new().borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightBlue));
    frame.render_stateful_widget(table, frame.size(), &mut app.habit_table_state);
}

fn handle_events(app: &mut UiApp) -> Result<bool> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
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

fn get_habit_vector(categories: &[(String, usize)], ids: &[usize]) -> Vec<bool> {
    let mut v = vec![];
    for (_, cat_id) in categories {
        v.push(ids.contains(cat_id));
    }
    v
}

fn get_daily_habit_rows<'a>(app: &UiApp) -> Vec<Row<'a>> {
    let categories = &app.header;
    let mut rows = vec![];
    for data_row in &app.habit_rows {
        let mut cells = vec![Cell::new(data_row.0.to_string())];
        if let Some(habit_vector) = &data_row.1 {
            for val in habit_vector {
                if *val {
                    cells.push(Cell::new("✓"));
                } else {
                    cells.push(Cell::new(""));
                }
            }
        } else {
            for _i in 0..categories.len() {
                cells.push(Cell::new("?"));
            }
        }
        rows.push(Row::new(cells));
    }
    rows
}
