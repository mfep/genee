mod habit_day_list_widget;

use std::io::stdout;

use crate::CliOptions;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use genee::datafile::{self, DiaryDataConnection};
use ratatui::prelude::*;

use self::habit_day_list_widget::{HabitDayListWidget, HabitDayListWidgetContext};

pub fn run_app(opts: &CliOptions) -> Result<()> {
    let mut app = UiApp::new(opts)?;

    crossterm::terminal::enable_raw_mode()?;
    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            app.habit_day_list_widget.render_habit_table(frame);
        })?;
        should_quit = handle_events(&mut app)?;
    }

    crossterm::terminal::disable_raw_mode()?;
    stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

struct UiApp {
    datafile: Box<dyn DiaryDataConnection>,
    habit_day_list_widget: HabitDayListWidget,
}

impl UiApp {
    fn new(opts: &CliOptions) -> Result<Self> {
        let mut datafile = datafile::open_datafile(opts.datafile.as_ref().unwrap())?;
        let habit_day_list_widget =
            HabitDayListWidget::new(HabitDayListWidgetContext::new(&mut *datafile))?;
        Ok(UiApp {
            datafile,
            habit_day_list_widget,
        })
    }
}

fn handle_events(app: &mut UiApp) -> Result<bool> {
    if event::poll(std::time::Duration::from_millis(100))? {
        let event = event::read()?;
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
        app.habit_day_list_widget
            .handle_events(HabitDayListWidgetContext::new(&mut *app.datafile), &event)?;
    }
    Ok(false)
}
