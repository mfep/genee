use ratatui::{prelude::*, widgets::*};

pub fn get_table_header<'a>(header: &[(String, usize)], first: &'a str) -> Row<'a> {
    let mut cells = vec![Cell::new(first)];
    for (name, _idx) in header {
        cells.push(Cell::new(name.clone()));
    }
    Row::new(cells).add_modifier(Modifier::BOLD)
}

pub fn decode_habit_vector(categories: &[(String, usize)], ids: &[usize]) -> Vec<bool> {
    let mut v = vec![];
    for (_, cat_id) in categories {
        v.push(ids.contains(cat_id));
    }
    v
}

pub fn encode_habit_vector(categories: &[(String, usize)], entries: &[bool]) -> Vec<usize> {
    assert_eq!(categories.len(), entries.len());
    let mut entry_ids = vec![];
    for (val, (_name, cat_id)) in entries.iter().zip(categories.iter()) {
        if *val {
            entry_ids.push(*cat_id);
        }
    }
    entry_ids
}
