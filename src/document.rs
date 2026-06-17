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

    pub fn clear(&mut self) {
        self.strings.clear();
        self.annotation = None;
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
            let mut visual_width = 0;
            while visual_width < wrap_width && current_col + chunk_len < self.columns.len() {
                let col_idx = current_col + chunk_len;
                visual_width += 1; // 1 for the column itself
                
                // Add separator if applicable
                let (_, box_end) = self.box_range(col_idx);
                if !self.columns[col_idx].is_barline(self.tuning.len())
                    && col_idx + 1 == box_end 
                    && col_idx + 1 < self.columns.len() 
                {
                    let next_col = col_idx + 1;
                    if !self.columns[next_col].is_barline(self.tuning.len()) {
                        visual_width += 1; // +1 for separator '-'
                    }
                }
                
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

    pub fn measure_start_col(&self, col: usize) -> usize {
        let mut start = col;
        let num_strings = self.tuning.len();
        while start > 0 && !self.columns[start - 1].is_barline(num_strings) {
            start -= 1;
        }
        start
    }

    pub fn box_range(&self, col: usize) -> (usize, usize) {
        let num_strings = self.tuning.len();
        if col >= self.columns.len() {
            return (col, col);
        }
        if self.columns[col].is_barline(num_strings) {
            return (col, col + 1);
        }
        let m_start = self.measure_start_col(col);
        let offset = col - m_start;
        let box_idx = offset / 3;
        let box_start = m_start + box_idx * 3;
        
        let mut box_end = box_start + 3;
        for i in box_start..box_end {
            if i < self.columns.len() && self.columns[i].is_barline(num_strings) {
                box_end = i;
                break;
            }
        }
        (box_start, box_end.min(self.columns.len()))
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

            let get_visual_offset = |col_idx_in_chunk: usize| -> usize {
                let mut offset = 2; // "e|"
                for j in 0..col_idx_in_chunk {
                    offset += 1;
                    let g_col = chunk_range.start + j;
                    let (_, box_end) = self.box_range(g_col);
                    if g_col + 1 == box_end && j + 1 < chunk.len() {
                        let next_g_col = g_col + 1;
                        if !self.columns[next_g_col].is_barline(self.tuning.len()) {
                            offset += 1;
                        }
                    }
                }
                offset
            };

            for (i, col) in chunk.iter().enumerate() {
                let global_col = chunk_range.start + i;
                let offset_i = get_visual_offset(i);

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
                for (i, col) in chunk.iter().enumerate() {
                    out.push(col.get_char(string_idx));
                    
                    let global_col = chunk_range.start + i;
                    let (_, box_end) = self.box_range(global_col);
                    if !self.columns[global_col].is_barline(self.tuning.len())
                        && global_col + 1 == box_end 
                        && i + 1 < chunk.len() 
                    {
                        let next_global_col = global_col + 1;
                        if !self.columns[next_global_col].is_barline(self.tuning.len()) {
                            out.push('-'); // separator
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_range() {
        let doc = TabDocument::default();
        assert_eq!(doc.box_range(0), (0, 3));
        assert_eq!(doc.box_range(1), (0, 3));
        assert_eq!(doc.box_range(2), (0, 3));
        assert_eq!(doc.box_range(3), (3, 6));
        assert_eq!(doc.box_range(12), (12, 15));
        assert_eq!(doc.box_range(14), (12, 15));
        assert_eq!(doc.box_range(15), (15, 16));
        assert_eq!(doc.box_range(16), (16, 19));
        assert_eq!(doc.box_range(30), (28, 31));
        assert_eq!(doc.box_range(31), (31, 32));
    }

    #[test]
    fn test_measure_start_col() {
        let doc = TabDocument::default();
        assert_eq!(doc.measure_start_col(0), 0);
        assert_eq!(doc.measure_start_col(5), 0);
        assert_eq!(doc.measure_start_col(14), 0);
        assert_eq!(doc.measure_start_col(15), 0);
        assert_eq!(doc.measure_start_col(16), 16);
        assert_eq!(doc.measure_start_col(20), 16);
        assert_eq!(doc.measure_start_col(31), 16);
    }

    #[test]
    fn test_dump_to_string() {
        let doc = TabDocument::default();
        let dump = doc.dump_to_string(100);
        assert!(dump.contains("e|-------------------|-------------------|-------------------|-------------------||"));
        assert!(dump.contains("B|-------------------|-------------------|-------------------|-------------------||"));
    }
}
