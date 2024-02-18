use crate::datafile::{self, DiaryDataConnection};
use anyhow::Result;
use chrono::{Local, NaiveDate};
use ratatui::{prelude::*, style::Color, widgets::*};

const COLORS: [Color; 6] = [
    Color::LightCyan,
    Color::LightMagenta,
    Color::LightGreen,
    Color::LightRed,
    Color::LightBlue,
    Color::LightYellow,
];

fn get_color(idx: usize) -> Color {
    COLORS[idx % COLORS.len()]
}

pub struct HabitFrequencyTableWidget {
    header: Vec<(String, usize)>,
    range_size: usize,
    iters: usize,
    date_ranges: Vec<(NaiveDate, NaiveDate)>,
    data_counts: Vec<Vec<usize>>,
}

pub enum HabitFrequencyTableWidgetInput {
    SetBeginDate(NaiveDate),
}

impl HabitFrequencyTableWidget {
    pub fn new(
        datafile: &dyn DiaryDataConnection,
        range_size: usize,
        iters: usize,
    ) -> Result<HabitFrequencyTableWidget> {
        let header = datafile.get_header()?;
        let date_ranges = datafile::get_date_ranges(&Local::now().date_naive(), range_size, iters);
        let data_counts = datafile.calculate_data_counts_per_iter(&date_ranges)?;
        Ok(HabitFrequencyTableWidget {
            header,
            range_size,
            iters,
            date_ranges,
            data_counts,
        })
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let inner_area = area.inner(&Margin::new(1, 1));
        frame.render_widget(Block::default().borders(Borders::ALL), area);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Min(0)])
            .split(inner_area);

        let date_list_text: Vec<Span> = self
            .date_ranges
            .iter()
            .enumerate()
            .map(|(idx, (from, to))| {
                Span::styled(
                    format!("{} - {} ", to, from),
                    Style::default().fg(get_color(idx)),
                )
            })
            .collect();
        let date_list_text = Line::from(date_list_text);
        frame.render_widget(Paragraph::new(date_list_text), inner_chunks[0]);

        let mut bar_chart = BarChart::default()
            .direction(Direction::Horizontal)
            .bar_gap(0)
            .bar_width(1)
            .group_gap(1);
        for (idx, (name, _id)) in self.header.iter().enumerate() {
            let bars: Vec<Bar> = self
                .data_counts
                .iter()
                .enumerate()
                .map(|(bar_idx, count_values)| {
                    let label = if bar_idx == 0 { name.as_str() } else { "" };
                    let count_value = count_values[idx];
                    let count_text = format!("{:2}", count_value);
                    Bar::default()
                        .value(count_value as u64)
                        .text_value(count_text)
                        .label(Line::from(label))
                        .style(Style::default().fg(get_color(bar_idx)))
                })
                .collect();
            let bar_group = BarGroup::default().bars(&bars);
            bar_chart = bar_chart.data(bar_group);
        }
        frame.render_widget(bar_chart, inner_chunks[1]);
    }

    pub fn update(
        &mut self,
        datafile: &dyn DiaryDataConnection,
        input: HabitFrequencyTableWidgetInput,
    ) -> Result<()> {
        match input {
            HabitFrequencyTableWidgetInput::SetBeginDate(date) => {
                self.date_ranges = datafile::get_date_ranges(&date, self.range_size, self.iters);
                self.data_counts = datafile.calculate_data_counts_per_iter(&self.date_ranges)?;
            }
        }
        Ok(())
    }
}
