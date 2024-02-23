mod habit_day_list_widget;
mod habit_frequency_table_widget;

use std::{fmt::Display, io::stdout};

use crate::CliOptions;
use anyhow::Result;
use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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

#[derive(Clone, Copy, PartialEq)]
enum Scale {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    FiveYearly,
}

impl Scale {
    fn smaller(&self) -> Scale {
        match self {
            Scale::Weekly => Scale::Weekly,
            Scale::Monthly => Scale::Weekly,
            Scale::Quarterly => Scale::Monthly,
            Scale::Yearly => Scale::Monthly,
            Scale::FiveYearly => Scale::Yearly,
        }
    }

    fn larger(&self) -> Scale {
        match self {
            Scale::Weekly => Scale::Monthly,
            Scale::Monthly => Scale::Quarterly,
            Scale::Quarterly => Scale::Yearly,
            Scale::Yearly => Scale::FiveYearly,
            Scale::FiveYearly => Scale::FiveYearly,
        }
    }

    fn value(&self) -> usize {
        match self {
            Scale::Weekly => 7,
            Scale::Monthly => 30,
            Scale::Quarterly => 90,
            Scale::Yearly => 365,
            Scale::FiveYearly => 1825,
        }
    }
}

impl Display for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scale::Weekly => f.write_str("weekly"),
            Scale::Monthly => f.write_str("monthly"),
            Scale::Quarterly => f.write_str("quarterly"),
            Scale::Yearly => f.write_str("yearly"),
            Scale::FiveYearly => f.write_str("five yearly"),
        }
    }
}

impl UiApp {
    fn new(opts: &CliOptions) -> Result<Self> {
        let datafile = datafile::open_datafile(opts.datafile.as_ref().unwrap())?;
        let habit_day_list_widget = HabitDayListWidget::new(&*datafile)?;
        let habit_frequency_table_widget = HabitFrequencyTableWidget::new(
            &*datafile,
            habit_day_list_widget.get_scale(),
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
                if key.kind != KeyEventKind::Press {
                    return Ok(false);
                }
                if key.code == KeyCode::Char('q') {
                    return Ok(true);
                }
                if key.code == KeyCode::Up && key.modifiers == KeyModifiers::NONE {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::StepEarlier)?;
                    self.update_frequency_table()?;
                } else if key.code == KeyCode::PageUp {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::StrideEarlier)?;
                    self.update_frequency_table()?;
                } else if key.code == KeyCode::Down && key.modifiers == KeyModifiers::NONE {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::StepLater)?;
                    self.update_frequency_table()?;
                } else if key.code == KeyCode::PageDown {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::StrideLater)?;
                    self.update_frequency_table()?;
                } else if key.code == KeyCode::Enter {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::SwitchMode)?;
                    self.habit_frequency_table_widget
                        .update(&*self.datafile, HabitFrequencyTableWidgetInput::DataChanged)?;
                } else if key.code == KeyCode::Left && key.modifiers == KeyModifiers::NONE {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateColumn(-1),
                    )?;
                } else if key.code == KeyCode::Right && key.modifiers == KeyModifiers::NONE {
                    self.habit_day_list_widget.update(
                        &mut *self.datafile,
                        HabitDayListWidgetInput::NavigateColumn(1),
                    )?;
                } else if key.code == KeyCode::Char(' ') {
                    self.habit_day_list_widget
                        .update(&mut *self.datafile, HabitDayListWidgetInput::SwitchValue)?;
                } else if key.code == KeyCode::Left && key.modifiers == KeyModifiers::CONTROL {
                    self.habit_frequency_table_widget.update(
                        &*self.datafile,
                        HabitFrequencyTableWidgetInput::SmallerScale,
                    )?;
                } else if key.code == KeyCode::Right && key.modifiers == KeyModifiers::CONTROL {
                    self.habit_frequency_table_widget
                        .update(&*self.datafile, HabitFrequencyTableWidgetInput::LargerScale)?;
                } else if key.code == KeyCode::Char('a') {
                    self.habit_frequency_table_widget.update(
                        &*self.datafile,
                        HabitFrequencyTableWidgetInput::FewerPeriods,
                    )?;
                } else if key.code == KeyCode::Char('s') {
                    self.habit_frequency_table_widget
                        .update(&*self.datafile, HabitFrequencyTableWidgetInput::MorePeriods)?;
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
