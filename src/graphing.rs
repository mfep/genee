use crate::datafile::{DiaryData, calculate_data_counts};
use anyhow::{bail, Result};
use chrono::{Local, Duration};

pub fn graph_last_n_days(data: &DiaryData, n: usize, graph_height: usize) -> Result<()> {
    if graph_height < 2 {
        bail!("Graph height must be at least 2");
    }
    let header = generate_header_string(&data.header);
    let today = Local::today().naive_local();
    let from_day = today.checked_sub_signed(Duration::days(n as i64)).unwrap();
    let counts = calculate_data_counts(data, &from_day, &today);
    let histogram = generate_histogram(&counts, graph_height);
    println!("Occurences in the last {} days:\n{}\n{}", n, header, histogram.unwrap_or(String::new()));
    Ok(())
}

fn generate_header_string(headers: &[String]) -> String {
    let mut result = String::new();
    for head in headers {
        result.push_str(format!("{:4.3}", head).as_str());
    }
    result
}

fn generate_histogram(counts: &[usize], graph_height: usize) -> Option<String> {
    let &max_count = counts.iter().max().unwrap();
    if max_count == 0usize {
        return None;
    }
    let mut ret = String::new();
    for &count in counts {
        ret += format!("{:<4}", count).as_str();
    }
    ret += "\n";
    let relative_heights : Vec<usize> = counts.iter()
        .map(|count| graph_height * count / max_count)
        .collect();
    for row in 0..graph_height {
        for &block_height in &relative_heights {
            if row < block_height {
                ret += "#   ";
            } else {
                ret += "    ";
            }
        }
        ret += "\n";
    }
    Some(ret)
}

#[test]
fn test_generate_header_string() {
    let header_string = generate_header_string(&[
        String::from("oreg"),
        String::from("allat"),
        String::from("a"),
        String::from("ac"),
    ]);
    assert_eq!("ore all a   ac  ", header_string);
}
