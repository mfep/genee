mod habit_day_list_widget;
mod habit_frequency_table_widget;

use std::io::stdout;

use crate::CliOptions;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use genee::datafile::{self, DiaryDataConnection};
use ratatui::prelude::*;

use self::{
    habit_day_list_widget::{HabitDayListWidget, HabitDayListWidgetContext},
    habit_frequency_table_widget::HabitFrequencyTableWidget,
};

pub fn run_app(opts: &CliOptions) -> Result<()> {
    let mut app = UiApp::new(opts)?;

    crossterm::terminal::enable_raw_mode()?;
    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            app.render(frame);
        })?;
        should_quit = app.handle_events()?;
    }

    crossterm::terminal::disable_raw_mode()?;
    stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

struct UiApp {
    datafile: Box<dyn DiaryDataConnection>,
    habit_day_list_widget: HabitDayListWidget,
    habit_frequency_table_widget: HabitFrequencyTableWidget,
}

impl UiApp {
    fn new(opts: &CliOptions) -> Result<Self> {
        let mut datafile = datafile::open_datafile(opts.datafile.as_ref().unwrap())?;
        let habit_day_list_widget =
            HabitDayListWidget::new(HabitDayListWidgetContext::new(&mut *datafile))?;
        let habit_frequency_table_widget = HabitFrequencyTableWidget::new(
            &*datafile,
            opts.graph_days.unwrap(),
            opts.past_periods.unwrap(),
        )?;
        Ok(UiApp {
            datafile,
            habit_day_list_widget,
            habit_frequency_table_widget,
        })
    }

    fn handle_events(&mut self) -> Result<bool> {
        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(true);
                }
            }
            self.habit_day_list_widget
                .handle_events(HabitDayListWidgetContext::new(&mut *self.datafile), &event)?;
        }
        Ok(false)
    }

    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.size());
        self.habit_day_list_widget.render(frame, chunks[0]);
        self.habit_frequency_table_widget.render(frame, chunks[1]);
    }
}
