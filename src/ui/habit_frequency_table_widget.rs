use crate::{
    CliOptions,
    datafile::{self, DiaryDataConnection},
};
use anyhow::Result;
use chrono::NaiveDate;
use ratatui::{prelude::*, style::Color, widgets::*};

use super::Scale;

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
    begin_date: NaiveDate,
    scale: Scale,
    iters: usize,
    date_ranges: Vec<(NaiveDate, NaiveDate)>,
    data_counts: Vec<Vec<usize>>,
}

pub enum HabitFrequencyTableWidgetInput {
    SetBeginDate(NaiveDate),
    SmallerScale,
    LargerScale,
    FewerPeriods,
    MorePeriods,
    DataChanged,
}

impl HabitFrequencyTableWidget {
    pub fn new(
        datafile: &dyn DiaryDataConnection,
        begin_date: NaiveDate,
        opts: &CliOptions,
        scale: Scale,
    ) -> Result<HabitFrequencyTableWidget> {
        let header = datafile.get_header()?;
        let mut result = HabitFrequencyTableWidget {
            header,
            scale,
            iters: opts.past_periods.unwrap(),
            begin_date,
            date_ranges: vec![],
            data_counts: vec![],
        };
        result.recalculate(datafile)?;
        Ok(result)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let inner_area = area.inner(Margin::new(1, 1));
        frame.render_widget(
            Block::bordered()
                .title_top(self.title())
                .title_bottom("Change scale: <Ctrl> + <←><→> Change periods: <a><s>"),
            area,
        );

        const DATE_RANGE_CHAR_COUNT: u16 = 24; // "2024-01-29 - 2024-02-27 "
        let date_range_num_chars = self.date_ranges.len() as u16 * DATE_RANGE_CHAR_COUNT;
        let date_range_lines = date_range_num_chars.div_ceil(inner_area.width);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(date_range_lines), Constraint::Min(0)])
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
        frame.render_widget(
            Paragraph::new(date_list_text)
                .wrap(Wrap { trim: true })
                .style(Style::default().bold()),
            inner_chunks[0],
        );

        let mut bar_chart = BarChart::default()
            .direction(Direction::Horizontal)
            .bar_gap(0)
            .bar_width(1)
            .group_gap(1)
            .max(self.scale.value() as u64);
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
                if date != self.begin_date {
                    self.begin_date = date;
                    self.recalculate(datafile)?;
                }
            }
            HabitFrequencyTableWidgetInput::SmallerScale => {
                self.scale = self.scale.smaller();
                self.recalculate(datafile)?;
            }
            HabitFrequencyTableWidgetInput::LargerScale => {
                self.scale = self.scale.larger();
                self.recalculate(datafile)?;
            }
            HabitFrequencyTableWidgetInput::FewerPeriods => {
                self.iters = usize::max(1usize, self.iters - 1);
                self.recalculate(datafile)?;
            }
            HabitFrequencyTableWidgetInput::MorePeriods => {
                self.iters = usize::max(1usize, self.iters + 1);
                self.recalculate(datafile)?;
            }
            HabitFrequencyTableWidgetInput::DataChanged => {
                self.recalculate(datafile)?;
            }
        }
        Ok(())
    }

    pub fn get_range(&self) -> (NaiveDate, NaiveDate) {
        (
            self.date_ranges.last().unwrap().1,
            self.date_ranges.first().unwrap().0,
        )
    }

    pub fn update_opts(&self, opts: &mut CliOptions) {
        opts.past_periods = Some(self.iters);
    }

    fn recalculate(&mut self, datafile: &dyn DiaryDataConnection) -> Result<()> {
        self.date_ranges =
            datafile::get_date_ranges(&self.begin_date, self.scale.value(), self.iters);
        self.data_counts = datafile.calculate_data_counts_per_iter(&self.date_ranges)?;
        Ok(())
    }

    fn title(&self) -> String {
        format!(
            "Habit histogram: {} {} periods until {}",
            self.iters, self.scale, self.begin_date
        )
    }
}
