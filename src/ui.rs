mod habit_day_list_widget;
mod habit_frequency_table_widget;

use std::io::stdout;

use crate::CliOptions;
use anyhow::Result;
use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use genee::datafile::{self, DiaryDataConnection};
use ratatui::prelude::*;

use self::{
    habit_day_list_widget::{HabitDayListWidget, HabitDayListWidgetInput},
    habit_frequency_table_widget::{HabitFrequencyTableWidget, HabitFrequencyTableWidgetInput},
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
        let datafile = datafile::open_datafile(opts.datafile.as_ref().unwrap())?;
        let habit_day_list_widget = HabitDayListWidget::new(&*datafile)?;
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
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Up {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateDate(1),
                    )?;
                    self.update_frequency_table()?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::PageUp {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateDate(
                            self.habit_day_list_widget
                                .get_render_height()
                                .unwrap_or_default() as isize,
                        ),
                    )?;
                    self.update_frequency_table()?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Down {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateDate(-1),
                    )?;
                    self.update_frequency_table()?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::PageDown {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateDate(
                            -(self
                                .habit_day_list_widget
                                .get_render_height()
                                .unwrap_or_default() as isize),
                        ),
                    )?;
                    self.update_frequency_table()?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::SwitchMode)?;
                    self.update_frequency_table()?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Left {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateColumn(-1),
                    )?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Right {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateColumn(1),
                    )?;
                } else if key.kind == KeyEventKind::Press && key.code == KeyCode::Char(' ') {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::SwitchValue)?;
                }
            }
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

    fn update_frequency_table(&mut self) -> Result<()> {
        let selected_date = self
            .habit_day_list_widget
            .get_selected_date()
            .unwrap_or_else(|| Local::now().date_naive());
        self.habit_frequency_table_widget.update(
            &*self.datafile,
            HabitFrequencyTableWidgetInput::SetBeginDate(selected_date),
        )?;
        Ok(())
    }
}
