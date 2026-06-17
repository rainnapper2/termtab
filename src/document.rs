use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabColumn {
    pub strings: [char; 6],
    pub annotation: Option<String>,
}

impl TabColumn {
    pub fn new() -> Self {
        Self {
            strings: ['-'; 6],
            annotation: None,
        }
    }

    pub fn barline() -> Self {
        Self {
            strings: ['|'; 6],
            annotation: None,
        }
    }

    pub fn is_blank(&self) -> bool {
        self.strings.iter().all(|&c| c == '-') && self.annotation.is_none()
    }

    pub fn is_barline(&self) -> bool {
        self.strings.iter().all(|&c| c == '|')
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
    pub tuning: [char; 6],
    pub clipboard: Vec<TabColumn>,
}

impl TabDocument {
    pub fn new() -> Self {
        // Start with 4 measures of 15 empty columns each
        let mut columns = Vec::new();
        for i in 0..4 {
            for _ in 0..15 {
                columns.push(TabColumn::new());
            }
            if i == 3 {
                // Last measure ends with a double barline
                columns.push(TabColumn::barline());
                columns.push(TabColumn::barline());
            } else {
                columns.push(TabColumn::barline());
            }
        }
        
        Self {
            columns,
            // Standard guitar tuning (high e to low E). 
            // 0 = highest string (e), 5 = lowest string (E)
            tuning: ['e', 'B', 'G', 'D', 'A', 'E'],
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
                    if self.columns[prev_idx].is_barline() && self.columns[curr_idx].is_barline() {
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
            if self.columns[i].is_barline() {
                if i == 0 || !self.columns[i - 1].is_barline() {
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
        self.columns[col_idx - 1].is_barline() && !self.columns[col_idx].is_barline()
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

            for string_idx in 0..6 {
                let tuning_char = self.tuning[string_idx];
                out.push_str(&format!("{}|", tuning_char));
                for col in chunk {
                    out.push(col.strings[string_idx]);
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
        Self::new()
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
