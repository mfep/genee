use crate::datafile::DiaryData;
use anyhow::{bail, Result};

pub fn graph_last_n_days(data: &DiaryData, n: usize, graph_height: usize) -> Result<()> {
    if graph_height < 2 {
        bail!("Graph height must be at least 2");
    }
    let header = generate_header_string(&data.header);
    
    Ok(())
}

fn generate_header_string(headers: &[String]) -> String {
    let mut result = String::new();
    for head in headers {
        result.push_str(format!("{:4.3}", head).as_str());
    }
    result
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
