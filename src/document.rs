use serde::{Serialize, Deserialize};

use std::collections::BTreeMap;
use serde::{Deserializer};

fn deserialize_sparse_strings<'de, D>(deserializer: D) -> Result<std::collections::BTreeMap<usize, char>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    let mut map = std::collections::BTreeMap::new();
    
    match value {
        serde_json::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                if let Some(s) = val.as_str() {
                    let c = s.chars().next().unwrap_or('-');
                    if c != '-' {
                        map.insert(i, c);
                    }
                }
            }
        }
        serde_json::Value::Object(obj) => {
            for (k, v) in obj {
                if let Ok(idx) = k.parse::<usize>() {
                    if let Some(s) = v.as_str() {
                        let c = s.chars().next().unwrap_or('-');
                        if c != '-' {
                            map.insert(idx, c);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    
    Ok(map)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabColumn {
    #[serde(deserialize_with = "deserialize_sparse_strings", skip_serializing_if = "BTreeMap::is_empty", default)]
    pub strings: BTreeMap<usize, char>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub annotation: Option<String>,
}

impl TabColumn {
    pub fn new() -> Self {
        Self {
            strings: BTreeMap::new(),
            annotation: None,
        }
    }

    pub fn barline(num_strings: usize) -> Self {
        let mut strings = BTreeMap::new();
        for i in 0..num_strings {
            strings.insert(i, '|');
        }
        Self {
            strings,
            annotation: None,
        }
    }

    pub fn get_char(&self, idx: usize) -> char {
        self.strings.get(&idx).copied().unwrap_or('-')
    }

    pub fn set_char(&mut self, idx: usize, c: char) {
        if c == '-' {
            self.strings.remove(&idx);
        } else {
            self.strings.insert(idx, c);
        }
    }

    pub fn is_blank(&self) -> bool {
        self.strings.is_empty() && self.annotation.is_none()
    }

    pub fn is_barline(&self, num_strings: usize) -> bool {
        self.strings.len() == num_strings && self.strings.values().all(|&c| c == '|')
    }
}

impl Default for TabColumn {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TabDocument {
    pub columns: Vec<TabColumn>,
    pub tuning: Vec<char>,
    pub clipboard: Vec<TabColumn>,
}

impl TabDocument {
    pub fn new(tuning: Vec<char>) -> Self {
        let num_strings = tuning.len();
        // Start with 4 measures of 15 empty columns each
        let mut columns = Vec::new();
        for i in 0..4 {
            for _ in 0..15 {
                columns.push(TabColumn::new());
            }
            if i == 3 {
                // Last measure ends with a double barline
                columns.push(TabColumn::barline(num_strings));
                columns.push(TabColumn::barline(num_strings));
            } else {
                columns.push(TabColumn::barline(num_strings));
            }
        }
        
        Self {
            columns,
            // Configurable tuning dynamically parsed
            tuning,
            clipboard: Vec::new(),
        }
    }

    pub fn calculate_chunks(&self, wrap_width: usize) -> Vec<std::ops::Range<usize>> {
        let mut chunks = Vec::new();
        let mut current_col = 0;
        
        while current_col < self.columns.len() {
            let mut chunk_len = 0;
            while chunk_len < wrap_width && current_col + chunk_len < self.columns.len() {
                chunk_len += 1;
                if chunk_len >= 2 {
                    let curr_idx = current_col + chunk_len - 1;
                    let prev_idx = curr_idx - 1;
                    if self.columns[curr_idx].is_barline(self.tuning.len()) && self.columns[prev_idx].is_barline(self.tuning.len()) {
                        break;
                    }
                }
            }
            let end_col = current_col + chunk_len;
            chunks.push(current_col..end_col);
            current_col += chunk_len;
        }
        chunks
    }

    pub fn measure_number_at_col(&self, col_idx: usize) -> usize {
        let mut measure = 1;
        for i in 0..col_idx {
            if self.columns[i].is_barline(self.tuning.len()) {
                if i == 0 || !self.columns[i - 1].is_barline(self.tuning.len()) {
                    measure += 1;
                }
            }
        }
        measure
    }

    pub fn is_measure_start(&self, col_idx: usize) -> bool {
        if col_idx == 0 {
            return true;
        }
        self.columns[col_idx - 1].is_barline(self.tuning.len()) && !self.columns[col_idx].is_barline(self.tuning.len())
    }

    pub fn dump_to_string(&self, wrap_width: usize) -> String {
        let mut out = String::new();
        let chunks = self.calculate_chunks(wrap_width);

        for chunk_range in chunks {
            let chunk = &self.columns[chunk_range.clone()];

            let mut measure_lines: Vec<Vec<char>> = Vec::new();
            let mut annotation_lines: Vec<Vec<char>> = Vec::new();

            let place_text = |text: &str, offset: usize, lines: &mut Vec<Vec<char>>| {
                let mut placed = false;
                for a_line in lines.iter_mut() {
                    let text_chars: Vec<char> = text.chars().collect();
                    while a_line.len() <= offset + text_chars.len() {
                        a_line.push(' ');
                    }
                    let is_free = a_line[offset..offset + text_chars.len()].iter().all(|&c| c == ' ');
                    if is_free {
                        for (j, &c) in text_chars.iter().enumerate() {
                            a_line[offset + j] = c;
                        }
                        placed = true;
                        break;
                    }
                }
                if !placed {
                    let mut new_line = vec![' '; offset];
                    let text_chars: Vec<char> = text.chars().collect();
                    new_line.extend(text_chars);
                    lines.push(new_line);
                }
            };

            for (i, col) in chunk.iter().enumerate() {
                let global_col = chunk_range.start + i;
                let offset_i = i + 2;

                if self.is_measure_start(global_col) {
                    let text = format!("[{}]", self.measure_number_at_col(global_col));
                    place_text(&text, offset_i, &mut measure_lines);
                }

                if let Some(text) = &col.annotation {
                    place_text(text, offset_i, &mut annotation_lines);
                }
            }

            for m_line in measure_lines {
                let s: String = m_line.into_iter().collect();
                out.push_str(&s);
                out.push('\n');
            }

            for a_line in annotation_lines {
                let s: String = a_line.into_iter().collect();
                out.push_str(&s);
                out.push('\n');
            }

            for string_idx in 0..self.tuning.len() {
                let tuning_char = self.tuning[string_idx];
                out.push_str(&format!("{}|", tuning_char));
                for col in chunk {
                    out.push(col.get_char(string_idx));
                }
                out.push('\n');
            }
            out.push('\n');
        }
        out
    }
}

impl Default for TabDocument {
    fn default() -> Self {
        Self::new(vec!['e', 'B', 'G', 'D', 'A', 'E'])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    pub col: usize,
    pub string: usize, // 0..=5
}

impl Cursor {
    pub fn new() -> Self {
        Self { col: 0, string: 0 }
    }
}
