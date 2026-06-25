use serde::{Serialize, Deserialize};
use crate::models::measure::{Measure, MeasureEnd, VirtualColumn};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabDocument {
    pub measures: Vec<Measure>,
    pub tuning: Vec<char>,
    #[serde(skip)]
    pub clipboard: Vec<Measure>,
}

impl TabDocument {
    pub fn invert_operation(&self, op: &crate::models::operation::Operation) -> crate::models::operation::Operation {
        use crate::models::operation::Operation::*;
        match op {
            InsertColumn { col, content } => DeleteColumn { col: *col, content: content.clone() },
            DeleteColumn { col, content } => InsertColumn { col: *col, content: content.clone() },
            SetChar { col, string, old_c, new_c } => SetChar { col: *col, string: *string, old_c: *new_c, new_c: *old_c },
            DeleteChar { col, string, old_c } => InsertChar { col: *col, string: *string, c: *old_c },
            InsertChar { col, string, c } => DeleteChar { col: *col, string: *string, old_c: *c },
            SetAnnotation { col, old_ann, new_ann } => SetAnnotation { col: *col, old_ann: new_ann.clone(), new_ann: old_ann.clone() },
            InsertBarline { col } => DeleteBarline { col: *col },
            DeleteBarline { col } => InsertBarline { col: *col },
            ReplaceChars { start_col, string, old_chars, new_chars } => ReplaceChars {
                start_col: *start_col, string: *string, old_chars: new_chars.clone(), new_chars: old_chars.clone()
            },
            DeleteColumnsRange { start, end, contents } => InsertColumnsRange { start: *start, contents: contents.clone() },
            InsertColumnsRange { start, contents } => DeleteColumnsRange { start: *start, end: *start + contents.len(), contents: contents.clone() },
        }
    }

    pub fn apply_operation(&mut self, op: &crate::models::operation::Operation) {
        use crate::models::operation::Operation::*;
        match op {
            InsertColumn { col, content } => {
                self.insert_col(*col);
                if !content.is_blank() {
                    for (i, &c) in content.strings.iter().enumerate() {
                        self.set_char(*col, i, c);
                    }
                    if let Some(ann) = &content.annotation {
                        self.set_annotation(*col, ann.clone());
                    }
                    if content.is_box_start {
                        self.set_box_start(*col, true);
                    }
                }
            }
            DeleteColumn { col, .. } => {
                self.delete_col(*col);
            }
            SetChar { col, string, new_c, .. } => {
                self.set_char(*col, *string, *new_c);
            }
            DeleteChar { col, string, .. } => {
                self.delete_char(*col, *string);
            }
            InsertChar { col, string, c } => {
                let (measure_idx, local_col) = self.global_col_to_local(*col);
                if let Some(measure) = self.measures.get_mut(measure_idx) {
                    if let Some(s) = measure.strings.get_mut(*string) {
                        if local_col <= s.len() {
                            s.insert(local_col, *c);
                            s.pop();
                        }
                    }
                }
            }
            SetAnnotation { col, new_ann, .. } => {
                self.set_annotation(*col, new_ann.clone().unwrap_or_default());
            }
            InsertBarline { col } => {
                self.insert_barline(*col);
            }
            DeleteBarline { col } => {
                // To delete a barline, we need to merge the measure at `col` with the previous one.
                // Or maybe just delete the column if it's an empty barline? 
                // We'll implement this if needed. Currently `delete_column_at` handles barlines.
                self.delete_col(*col);
            }
            ReplaceChars { start_col, string, new_chars, .. } => {
                for (i, &c) in new_chars.iter().enumerate() {
                    self.set_char(*start_col + i, *string, c);
                }
            }
            DeleteColumnsRange { start, end, .. } => {
                self.delete_cols(*start, *end);
            }
            InsertColumnsRange { start, contents } => {
                for (i, content) in contents.iter().enumerate() {
                    self.insert_col(*start + i);
                    if !content.is_blank() {
                        for (s, &c) in content.strings.iter().enumerate() {
                            self.set_char(*start + i, s, c);
                        }
                        if let Some(ann) = &content.annotation {
                            self.set_annotation(*start + i, ann.clone());
                        }
                        if content.is_box_start {
                            self.set_box_start(*start + i, true);
                        }
                    }
                }
            }
        }
    }

    pub fn new(tuning: Vec<char>) -> Self {
        let num_strings = tuning.len();
        let mut measures = Vec::new();
        for i in 0..4 {
            let mut m = Measure::new(num_strings, 15);
            if i == 3 {
                m.end_decorator = MeasureEnd::DoubleBar;
            }
            measures.push(m);
        }
        
        Self {
            measures,
            tuning,
            clipboard: Vec::new(),
        }
    }

    pub fn global_col_to_local(&self, global_col: usize) -> (usize, usize) {
        let mut curr = 0;
        for (m_idx, m) in self.measures.iter().enumerate() {
            let m_len = m.len();
            if global_col < curr + m_len {
                return (m_idx, global_col - curr);
            }
            if global_col == curr + m_len {
                return (m_idx, m_len);
            }
            curr += m_len + 1; // +1 for the barline
        }
        let last_m = self.measures.len().saturating_sub(1);
        (last_m, self.measures[last_m].len())
    }

    pub fn local_col_to_global(&self, measure_idx: usize, local_col: usize) -> usize {
        let mut curr = 0;
        for i in 0..measure_idx.min(self.measures.len()) {
            curr += self.measures[i].len() + 1;
        }
        curr + local_col
    }

    pub fn total_global_cols(&self) -> usize {
        let mut total = 0;
        for m in &self.measures {
            total += m.len() + 1; // +1 for barline
        }
        total
    }

    pub fn get_char(&self, global_col: usize, string_idx: usize) -> char {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return '-'; }
        if l_col == self.measures[m_idx].len() { return '|'; }
        self.measures[m_idx].get_char(string_idx, l_col)
    }

    pub fn delete_char(&mut self, global_col: usize, string_idx: usize) {
        let (measure_idx, local_col) = self.global_col_to_local(global_col);
        if let Some(measure) = self.measures.get_mut(measure_idx) {
            measure.delete_char(string_idx, local_col);
        }
    }

    pub fn set_char(&mut self, global_col: usize, string_idx: usize, c: char) {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return; }
        if l_col == self.measures[m_idx].len() { return; }
        self.measures[m_idx].set_char(string_idx, l_col, c);
    }

    pub fn is_blank(&self, global_col: usize) -> bool {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return true; }
        if l_col == self.measures[m_idx].len() { return false; } // barline not blank
        let m = &self.measures[m_idx];
        for s in 0..self.tuning.len() {
            if m.get_char(s, l_col) != '-' { return false; }
        }
        m.annotations.get(&l_col).is_none() && !m.box_starts.contains(&l_col)
    }

    pub fn is_barline(&self, global_col: usize) -> bool {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return false; }
        l_col == self.measures[m_idx].len()
    }

    pub fn insert_col(&mut self, global_col: usize) {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return; }
        if l_col == self.measures[m_idx].len() {
            // Cannot insert ON a barline. Insert at end of measure instead.
            self.measures[m_idx].insert_col(l_col);
        } else {
            self.measures[m_idx].insert_col(l_col);
        }
    }

    pub fn delete_col(&mut self, global_col: usize) {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return; }
        if l_col == self.measures[m_idx].len() {
            // Deleted a barline: merge measures!
            if m_idx + 1 < self.measures.len() {
                let next_m = self.measures.remove(m_idx + 1);
                let current_len = self.measures[m_idx].len();
                
                // Append strings
                for (s1, s2) in self.measures[m_idx].strings.iter_mut().zip(next_m.strings.iter()) {
                    s1.push_str(s2);
                }
                
                // Shift and merge annotations
                for (k, v) in next_m.annotations {
                    self.measures[m_idx].annotations.insert(k + current_len, v);
                }
                
                // Shift and merge box_starts
                for bs in next_m.box_starts {
                    self.measures[m_idx].box_starts.push(bs + current_len);
                }
                
                self.measures[m_idx].end_decorator = next_m.end_decorator;
            }
        } else {
            self.measures[m_idx].delete_col(l_col);
        }
    }

    pub fn append_col(&mut self) {
        if let Some(m) = self.measures.last_mut() {
            m.insert_col(m.len());
        }
    }

    pub fn insert_barline(&mut self, global_col: usize) {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() { return; }
        if l_col == self.measures[m_idx].len() { return; } // already barline
        let new_m = self.measures[m_idx].split_at(l_col);
        self.measures.insert(m_idx + 1, new_m);
    }

    pub fn set_annotation(&mut self, global_col: usize, text: String) {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() || l_col == self.measures[m_idx].len() { return; }
        self.measures[m_idx].annotations.insert(l_col, text);
    }

    pub fn get_annotation(&self, global_col: usize) -> Option<String> {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() || l_col == self.measures[m_idx].len() { return None; }
        self.measures[m_idx].annotations.get(&l_col).cloned()
    }

    pub fn is_measure_start(&self, global_col: usize) -> bool {
        let (_m_idx, l_col) = self.global_col_to_local(global_col);
        l_col == 0
    }

    pub fn measure_number_at_col(&self, global_col: usize) -> usize {
        let (m_idx, _) = self.global_col_to_local(global_col);
        m_idx + 1
    }

    pub fn is_box_start(&self, global_col: usize) -> bool {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() || l_col == self.measures[m_idx].len() { return false; }
        self.measures[m_idx].box_starts.contains(&l_col)
    }

    pub fn set_box_start(&mut self, global_col: usize, is_start: bool) {
        let (m_idx, l_col) = self.global_col_to_local(global_col);
        if m_idx >= self.measures.len() || l_col == self.measures[m_idx].len() { return; }
        if is_start {
            if !self.measures[m_idx].box_starts.contains(&l_col) {
                self.measures[m_idx].box_starts.push(l_col);
            }
        } else {
            self.measures[m_idx].box_starts.retain(|&x| x != l_col);
        }
    }
}

impl Default for TabDocument {
    fn default() -> Self {
        Self::new(vec!['e', 'B', 'G', 'D', 'A', 'E'])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    pub col: usize,
    pub string: usize, // 0..=5
}

impl Cursor {
    pub fn new() -> Self {
        Self { col: 0, string: 0 }
    }
}


impl TabDocument {
    pub fn get_virtual_column(&self, global_col: usize) -> VirtualColumn {
        let mut strings = Vec::new();
        for s in 0..self.tuning.len() {
            strings.push(self.get_char(global_col, s));
        }
        let is_barline = self.is_barline(global_col);
        VirtualColumn {
            strings,
            annotation: self.get_annotation(global_col),
            is_box_start: self.is_box_start(global_col),
            is_barline,
        }
    }

    pub fn get_virtual_columns(&self, range: std::ops::Range<usize>) -> Vec<VirtualColumn> {
        let mut cols = Vec::new();
        for i in range {
            cols.push(self.get_virtual_column(i));
        }
        cols
    }

    pub fn calculate_chunks(&self, wrap_width: usize) -> Vec<std::ops::Range<usize>> {
        let mut chunks = Vec::new();
        let mut current_col = 0;
        let total = self.total_global_cols();
        
        while current_col < total {
            let mut chunk_len = 0;
            while chunk_len < wrap_width && current_col + chunk_len < total {
                chunk_len += 1;
                if chunk_len >= 2 {
                    let curr_idx = current_col + chunk_len - 1;
                    let prev_idx = curr_idx - 1;
                    if self.is_barline(curr_idx) && self.is_barline(prev_idx) {
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

    pub fn copy_cols(&mut self, start: usize, end: usize) {
        // Simple implementation: Just store dummy measures in clipboard
        // Since copying partial measures into full measures is hard, we'll store them as 1 col measures
        self.clipboard.clear();
        for i in start..=end {
            let mut m = Measure::new(self.tuning.len(), 1);
            for s in 0..self.tuning.len() {
                m.set_char(s, 0, self.get_char(i, s));
            }
            if let Some(ann) = self.get_annotation(i) {
                m.annotations.insert(0, ann);
            }
            if self.is_box_start(i) {
                m.box_starts.push(0);
            }
            self.clipboard.push(m);
        }
    }

    pub fn delete_cols(&mut self, start: usize, end: usize) {
        for i in (start..=end).rev() {
            self.delete_col(i);
        }
    }

    pub fn paste_clipboard(&mut self, start: usize) {
        let mut offset = 0;
        let clip = self.clipboard.clone();
        for m in &clip {
            self.insert_col(start + offset);
            for s in 0..self.tuning.len() {
                self.set_char(start + offset, s, m.get_char(s, 0));
            }
            if let Some(ann) = m.annotations.get(&0) {
                self.set_annotation(start + offset, ann.clone());
            }
            if m.box_starts.contains(&0) {
                self.set_box_start(start + offset, true);
            }
            offset += 1;
        }
    }

    pub fn dump_to_string(&self, wrap_width: usize) -> String {
        let mut out = String::new();
        let chunks = self.calculate_chunks(wrap_width);

        for chunk_range in chunks {
            let chunk = self.get_virtual_columns(chunk_range.clone());

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
                for col in &chunk {
                    out.push(col.get_char(string_idx));
                }
                out.push('\n');
            }
            out.push('\n');
        }
        out
    }
}
