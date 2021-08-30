//! Functions for displaying habit data on the terminal.
use crate::datafile;
use crate::datafile::DiaryData;
use anyhow::{bail, Result};
use chrono::NaiveDate;
use yansi::{Color, Paint};

const COLORS: &[Color] = &[
    Color::Green,
    Color::Magenta,
    Color::Yellow,
    Color::Cyan,
    Color::Red,
];

/// Prints colored habit data sums to stdout.
pub fn graph_last_n_days(
    data: &DiaryData,
    last_date: &NaiveDate,
    period: usize,
    iters: usize,
    max_width: usize,
) -> Result<()> {
    if max_width < 10 {
        bail!("Graph height must be at least 10");
    }
    let date_ranges = datafile::get_date_ranges(last_date, period, iters);
    let count_vectors = datafile::calculate_data_counts_per_iter(data, &date_ranges);
    let rows = generate_rows(&data.header, &count_vectors, max_width)?;
    println!("{}{}", format_ranges(&date_ranges, max_width), rows);
    Ok(())
}

/// Prints a header and a single row in a nice tabular way.
pub fn pretty_print_diary_row(data: &DiaryData, date: &NaiveDate) -> String {
    pretty_print_diary_rows(data, date, date)
}

/// Prints the diary table with header between the begin and end date.
/// Both limits inclusive.
pub fn pretty_print_diary_rows(
    data: &DiaryData,
    begin_date: &NaiveDate,
    end_date: &NaiveDate,
) -> String {
    let mut ret = String::new();
    ret += &pretty_print_header(&data.header);
    let mut current_date = *begin_date;
    while &current_date <= end_date {
        let current_row = data.data.get(&current_date);
        if let Some(row) = current_row {
            ret += &pretty_print_row(&current_date, row);
        } else {
            ret += &format!(
                "{} !date missing from diary!\n",
                current_date.format(datafile::DATE_FORMAT)
            );
        }
        current_date += chrono::Duration::days(1);
    }
    ret
}

fn pretty_print_header(headers: &[String]) -> String {
    let mut ret = String::new();
    ret += "          ";
    for header in headers {
        ret += " ";
        ret += &match header.len() {
            0 => panic!("Empty header is not allowed"),
            1 => format!(" {} ", header),
            2 => format!(" {}", header),
            _ => header.split_at(3).0.to_string(),
        };
    }
    ret += "\n";
    ret
}

fn pretty_print_row(date: &NaiveDate, data: &[bool]) -> String {
    let mut ret = String::new();
    ret += &date.format(datafile::DATE_FORMAT).to_string();
    for &val in data {
        ret += if val { "  ✓ " } else { "    " };
    }
    ret += "\n";
    ret
}

fn generate_rows(
    names: &[String],
    count_vectors: &[Vec<usize>],
    max_width: usize,
) -> Result<String> {
    const BLOCK: &str = "▇";
    if count_vectors
        .iter()
        .any(|count_vector| count_vector.len() != names.len())
    {
        bail!("Input header length does not match count length");
    }
    let mut ret = String::new();
    let max_width = max_width - 8;
    let max_count = count_vectors.iter().flat_map(|vector| vector.iter()).max();
    if max_count.is_none() || *max_count.unwrap() == 0 {
        bail!("No input data");
    }
    let max_count = max_count.unwrap();
    for (name_index, data_name) in names.iter().enumerate() {
        for (vector_index, count_vector) in count_vectors.iter().enumerate() {
            let head = if vector_index == 0 {
                format!("{:<3.3} ", Paint::blue(data_name).italic())
            } else {
                String::from("    ")
            };
            ret += &head;

            let current_count = count_vector[name_index];
            let width = current_count * max_width / max_count;
            let color = COLORS[vector_index % COLORS.len()];
            if width == 0 {
                ret += &Paint::new("▏").fg(color).to_string();
            } else {
                for _ in 0..width {
                    ret += &Paint::new(BLOCK).fg(color).to_string();
                }
                ret += " ";
            }
            ret += &format!("{}\n", Paint::new(current_count).bold());
        }
    }
    Ok(ret)
}

fn format_ranges(date_ranges: &[(NaiveDate, NaiveDate)], max_width: usize) -> String {
    let mut ret = String::new();
    let mut current_line_length: usize = 0;
    for (index, (range_start, range_end)) in date_ranges.iter().enumerate() {
        let color = COLORS[index % COLORS.len()];
        let range_start = range_start.format(datafile::DATE_FORMAT);
        let range_end = range_end.format(datafile::DATE_FORMAT);
        let formatted_range = format!("{}→{}\t", range_start, range_end);
        if current_line_length > max_width {
            ret += "\n";
            current_line_length = 0;
        }
        ret += &Paint::new(&formatted_range).fg(color).to_string();
        current_line_length += formatted_range.len();
    }
    ret += "\n";
    ret
}
