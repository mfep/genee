//! Functions for displaying habit data on the terminal.
use crate::datafile;
use crate::datafile::DiaryData;
use anyhow::{bail, Result};
use chrono::Local;
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
    period: usize,
    iters: usize,
    max_width: usize,
) -> Result<()> {
    if max_width < 10 {
        bail!("Graph height must be at least 10");
    }
    let today = Local::today().naive_local();
    let count_vectors = datafile::calculate_data_counts_per_iter(data, &today, period, iters);
    let rows = generate_rows(&data.header, &count_vectors, max_width)?;
    println!("{}", rows);
    Ok(())
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
