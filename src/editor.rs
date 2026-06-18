use crate::document::{TabColumn, TabDocument, Cursor, DEFAULT_MEASURE_LEN};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Editor {
    pub document: TabDocument,
    pub cursor: Cursor,
    pub undo_stack: Vec<(TabDocument, Cursor)>,
    pub redo_stack: Vec<(TabDocument, Cursor)>,
}

impl Editor {
    pub fn new(tuning: Vec<char>) -> Self {
        Self {
            document: TabDocument::new(tuning),
            cursor: Cursor::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }



    /// Save the current state to the undo stack and clear redo stack.
    fn save_state(&mut self) {
        self.undo_stack.push((self.document.clone(), self.cursor));
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some((prev_doc, prev_cursor)) = self.undo_stack.pop() {
            self.redo_stack.push((self.document.clone(), self.cursor));
            self.document = prev_doc;
            self.cursor = prev_cursor;
        }
    }

    pub fn redo(&mut self) {
        if let Some((next_doc, next_cursor)) = self.redo_stack.pop() {
            self.undo_stack.push((self.document.clone(), self.cursor));
            self.document = next_doc;
            self.cursor = next_cursor;
        }
    }



    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        let new_string = (self.cursor.string as isize + dy).clamp(0, self.document.tuning.len().saturating_sub(1) as isize) as usize;
        
        let mut new_col = self.cursor.col;
        if dx != 0 {
            let direction = dx.signum();
            let steps = dx.abs();
            let num_strings = self.document.tuning.len();
            for _ in 0..steps {
                if direction > 0 {
                    new_col += 1;
                    while new_col < self.document.columns.len() && self.document.columns[new_col].is_barline(num_strings) {
                        new_col += 1;
                    }
                } else {
                    if new_col > 0 {
                        new_col -= 1;
                        while new_col > 0 && self.document.columns[new_col].is_barline(num_strings) {
                            new_col -= 1;
                        }
                    }
                }
            }
        }

        while new_col >= self.document.columns.len() {
            self.document.append_measure();
        }

        self.cursor.col = new_col;
        self.cursor.string = new_string;
    }

    pub fn jump_to_measure(&mut self, target_measure: usize) {
        for i in 0..self.document.columns.len() {
            if self.document.is_measure_start(i) && self.document.measure_number_at_col(i) == target_measure {
                self.cursor.col = i;
                return;
            }
        }
        self.cursor.col = self.document.columns.len().saturating_sub(1);
    }

    pub fn jump_next_measure(&mut self) {
        let num_strings = self.document.tuning.len();
        for i in (self.cursor.col + 1)..self.document.columns.len() {
            if self.document.columns[i].is_barline(num_strings) {
                self.cursor.col = i + 1;
                if self.cursor.col >= self.document.columns.len() {
                    self.document.append_measure();
                }
                return;
            }
        }
        self.cursor.col = self.document.columns.len().saturating_sub(1);
    }

    pub fn jump_prev_measure(&mut self) {
        if self.cursor.col == 0 { return; }
        let mut search_start = self.cursor.col.saturating_sub(1);
        
        // If we are already at the start of a measure (column right after a barline),
        // skip this barline to jump to the start of the previous measure.
        if self.document.columns[search_start].is_barline(self.document.tuning.len()) {
            search_start = search_start.saturating_sub(1);
        }

        for i in (0..=search_start).rev() {
            if self.document.columns[i].is_barline(self.document.tuning.len()) {
                self.cursor.col = i + 1;
                return;
            }
        }
        self.cursor.col = 0;
    }

    pub fn jump_end_measure(&mut self) {
        let mut start_search = self.cursor.col + 1;
        
        // If we are already at the end of a measure (column right before a barline),
        // skip the upcoming barline to jump to the end of the next measure.
        if start_search < self.document.columns.len() && self.document.columns[start_search].is_barline(self.document.tuning.len()) {
            start_search += 1;
        }

        for i in start_search..self.document.columns.len() {
            if self.document.columns[i].is_barline(self.document.tuning.len()) {
                self.cursor.col = i.saturating_sub(1);
                return;
            }
        }
        self.cursor.col = self.document.columns.len().saturating_sub(1);
    }

    pub fn jump_next_row(&mut self, wrap_width: usize) {
        let chunks = self.document.calculate_chunks(wrap_width);
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.contains(&self.cursor.col) {
                if i + 1 < chunks.len() {
                    let offset = self.cursor.col - chunk.start;
                    let next_chunk = &chunks[i + 1];
                    self.cursor.col = (next_chunk.start + offset).min(next_chunk.end.saturating_sub(1));
                }
                break;
            }
        }
    }

    pub fn jump_prev_row(&mut self, wrap_width: usize) {
        let chunks = self.document.calculate_chunks(wrap_width);
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.contains(&self.cursor.col) {
                if i > 0 {
                    let offset = self.cursor.col - chunk.start;
                    let prev_chunk = &chunks[i - 1];
                    self.cursor.col = (prev_chunk.start + offset).min(prev_chunk.end.saturating_sub(1));
                }
                break;
            }
        }
    }

    pub fn insert_box(&mut self) {
        self.save_state();
        let (box_start, _) = self.document.box_range(self.cursor.col);
        
        let mut col = TabColumn::new();
        col.is_box_start = true;
        self.document.columns.insert(box_start, col);
        self.cursor.col = box_start;
    }

    pub fn delete_box(&mut self) {
        if self.document.columns.is_empty() { return; }
        self.save_state();
        let (box_start, box_end) = self.document.box_range(self.cursor.col);
        
        let num_strings = self.document.tuning.len();
        if self.document.columns[self.cursor.col].is_barline(num_strings) {
            self.document.columns.remove(self.cursor.col);
        } else {
            self.document.columns.drain(box_start..box_end);
        }

        // Ensure there's always at least one column
        if self.document.columns.is_empty() {
            let mut col = TabColumn::new();
            col.is_box_start = true;
            self.document.columns.push(col);
        }
        if self.cursor.col >= self.document.columns.len() {
            self.cursor.col = self.document.columns.len() - 1;
        }
        if !self.document.columns[self.cursor.col].is_barline(num_strings) {
            let (new_box_start, _) = self.document.box_range(self.cursor.col);
            self.cursor.col = new_box_start;
        }
    }

    pub fn clear_box(&mut self) {
        let col = self.cursor.col;
        let num_strings = self.document.tuning.len();
        if self.document.columns[col].is_barline(num_strings) {
            return;
        }
        self.save_state();
        let (box_start, box_end) = self.document.box_range(col);
        for c in &mut self.document.columns[box_start..box_end] {
            c.clear();
        }
    }

    pub fn expand_active_box(&mut self) {
        let col = self.cursor.col;
        let num_strings = self.document.tuning.len();
        if self.document.columns[col].is_barline(num_strings) {
            return;
        }
        self.save_state();
        let (_, box_end) = self.document.box_range(col);
        self.document.columns.insert(box_end, TabColumn::new());
    }

    pub fn shrink_active_box(&mut self) {
        let col = self.cursor.col;
        let num_strings = self.document.tuning.len();
        if self.document.columns[col].is_barline(num_strings) {
            return;
        }
        let (box_start, box_end) = self.document.box_range(col);
        let size = box_end - box_start;
        if size <= 1 {
            return;
        }
        self.save_state();
        self.document.columns.remove(box_end - 1);
        if self.cursor.col >= box_end - 1 {
            self.cursor.col = box_end - 2;
        }
    }

    pub fn insert_char_in_box(&mut self, c: char) -> Result<(), &'static str> {
        let col = self.cursor.col;
        let num_strings = self.document.tuning.len();
        if col < self.document.columns.len() && self.document.columns[col].is_barline(num_strings) {
            return Err("Cannot insert inside a barline");
        }
        
        self.save_state();
        
        if col >= self.document.columns.len() {
            self.document.append_measure();
        }
        
        let was_start = self.document.columns[col].is_box_start;
        self.document.columns[col].is_box_start = false;
        
        let mut new_col = TabColumn::new();
        new_col.is_box_start = was_start;
        new_col.set_char(self.cursor.string, c);
        
        self.document.columns.insert(col, new_col);
        self.cursor.col += 1;
        Ok(())
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor.col == 0 { return; }
        let prev_col = self.cursor.col - 1;
        let num_strings = self.document.tuning.len();
        if self.document.columns[prev_col].is_barline(num_strings) {
            return; // Don't delete barlines with backspace
        }
        
        self.save_state();
        let was_start = self.document.columns[prev_col].is_box_start;
        self.document.columns.remove(prev_col);
        
        if was_start && prev_col < self.document.columns.len() {
            if !self.document.columns[prev_col].is_barline(num_strings) {
                self.document.columns[prev_col].is_box_start = true;
            }
        }
        
        self.cursor.col = prev_col;
    }

    pub fn replace_chars(&mut self, chars: &[char]) {
        self.save_state();
        
        // Ensure we have enough columns to fit the characters
        while self.cursor.col + chars.len() > self.document.columns.len() {
            self.document.columns.push(TabColumn::new());
        }

        for (i, &c) in chars.iter().enumerate() {
            self.document.columns[self.cursor.col + i].set_char(self.cursor.string, c);
        }
    }

    pub fn insert_barline(&mut self) -> Result<(), &'static str> {
        let is_blank = self.document.columns[self.cursor.col].is_blank();
        if !is_blank {
            return Err("Column must be completely blank to insert a barline");
        }

        self.save_state();
        self.document.columns[self.cursor.col] = TabColumn::barline(self.document.tuning.len());
        Ok(())
    }

    pub fn set_annotation(&mut self, text: String) {
        self.save_state();
        self.document.columns[self.cursor.col].annotation = Some(text);
    }

    pub fn copy_columns(&mut self, start: usize, end: usize) {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let e = e.min(self.document.columns.len().saturating_sub(1));
        
        self.document.clipboard = self.document.columns[s..=e].to_vec();
    }

    pub fn delete_columns_range(&mut self, start: usize, end: usize) {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let e = e.min(self.document.columns.len().saturating_sub(1));
        
        self.save_state();
        self.document.columns.drain(s..=e);
        
        if self.document.columns.is_empty() {
            self.document.columns.push(TabColumn::new());
        }
        
        self.cursor.col = s.min(self.document.columns.len() - 1);
    }

    pub fn paste_columns(&mut self) {
        if self.document.clipboard.is_empty() {
            return;
        }
        self.save_state();
        let clip = self.document.clipboard.clone();
        let num_strings = self.document.tuning.len();
        let col = self.cursor.col;
        
        // 1. Insert clip at col
        let tail = self.document.columns.split_off(col);
        self.document.columns.extend(clip);
        self.document.columns.extend(tail);
        
        // 2. Ripple overflow
        let m_len = DEFAULT_MEASURE_LEN;
        let mut current_col = col;
        
        loop {
            let m_start = self.document.measure_start_col(current_col);
            let mut m_end = m_start;
            while m_end < self.document.columns.len() && !self.document.columns[m_end].is_barline(num_strings) {
                m_end += 1;
            }
            
            if m_end == self.document.columns.len() {
                let current_len = self.document.columns.len() - m_start;
                if current_len > m_len {
                    let split_pt = m_start + m_len;
                    let overflow = self.document.columns.split_off(split_pt);
                    self.document.columns.push(TabColumn::barline(num_strings));
                    self.document.columns.extend(overflow);
                    current_col = split_pt + 1;
                    continue;
                } else {
                    let pad_len = m_len - current_len;
                    self.document.columns.extend(vec![TabColumn::new(); pad_len]);
                    self.document.columns.push(TabColumn::barline(num_strings));
                    self.document.columns.push(TabColumn::barline(num_strings));
                    break;
                }
            }
            
            let current_measure_len = m_end - m_start;
            if current_measure_len > m_len {
                let split_pt = m_start + m_len;
                let overflow_range = split_pt .. m_end;
                let overflow_cols: Vec<TabColumn> = self.document.columns[overflow_range.clone()].to_vec();
                
                self.document.columns.drain(overflow_range);
                
                let mut insert_pos = split_pt + 1;
                let mut barline_count = 1;
                while insert_pos < self.document.columns.len() && self.document.columns[insert_pos].is_barline(num_strings) {
                    insert_pos += 1;
                    barline_count += 1;
                }
                
                if barline_count > 1 {
                    // Convert double/multiple barlines to single when pushing content past them
                    let num_to_remove = barline_count - 1;
                    self.document.columns.drain(split_pt + 1 .. split_pt + 1 + num_to_remove);
                    insert_pos -= num_to_remove;
                }
                
                let tail = self.document.columns.split_off(insert_pos);
                self.document.columns.extend(overflow_cols);
                self.document.columns.extend(tail);
                
                current_col = insert_pos;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_insert_delete_box() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        assert_eq!(ed.document.columns.len(), 69); // 4 measures * 16 cols + 5 barlines = 69
        ed.insert_box();
        assert_eq!(ed.document.columns.len(), 70);
        ed.undo();
        assert_eq!(ed.document.columns.len(), 69);
        ed.redo();
        assert_eq!(ed.document.columns.len(), 70);

        ed.delete_box();
        assert_eq!(ed.document.columns.len(), 69);
    }

    #[test]
    fn test_editor_navigation() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        
        // Move right: should go to col 1
        ed.move_cursor(1, 0);
        assert_eq!(ed.cursor.col, 1);
        
        // Move right again: should go to col 2
        ed.move_cursor(1, 0);
        assert_eq!(ed.cursor.col, 2);
        
        // Move left: should go to col 1
        ed.move_cursor(-1, 0);
        assert_eq!(ed.cursor.col, 1);

        // Move to end of measure 1 (col 15)
        ed.cursor.col = 15;
        // Move right: should go to col 17 (barline at 16)
        ed.move_cursor(1, 0);
        assert_eq!(ed.cursor.col, 17);
        
        // Move left from 17: should go to 15 (barline at 16)
        ed.move_cursor(-1, 0);
        assert_eq!(ed.cursor.col, 15);
    }

    #[test]
    fn test_editor_replace_chars() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['1', '1']);
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.document.columns[1].get_char(0), '1');
        assert_eq!(ed.document.columns[2].get_char(0), '-');

        ed.undo();
        assert_eq!(ed.document.columns[0].get_char(0), '-');
        assert_eq!(ed.document.columns[1].get_char(0), '-');
    }

    #[test]
    fn test_editor_copy_paste() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['9', '9', '9']);
        
        ed.copy_columns(0, 2);
        assert_eq!(ed.document.clipboard.len(), 3);
        assert_eq!(ed.document.clipboard[0].get_char(0), '9');

        ed.cursor.col = 6;
        ed.paste_columns();
        
        // M1: cols 0..5 (original) + clip (3) + cols 6..12 (original) = 16 cols
        assert_eq!(ed.document.columns[0].get_char(0), '9');
        assert_eq!(ed.document.columns[1].get_char(0), '9');
        assert_eq!(ed.document.columns[2].get_char(0), '9');
        assert_eq!(ed.document.columns[3].get_char(0), '-');
        assert_eq!(ed.document.columns[6].get_char(0), '9');
        assert_eq!(ed.document.columns[7].get_char(0), '9');
        assert_eq!(ed.document.columns[8].get_char(0), '9');
        assert_eq!(ed.document.columns[16].get_char(0), '|');
        
        // M2: cols 13..15 (original M1 overflow, 3 cols) + 13 cols of original M2 = 16 cols
        assert_eq!(ed.document.columns[17].get_char(0), '-');
        assert_eq!(ed.document.columns[33].get_char(0), '|');
        
        assert_eq!(ed.document.columns.len(), 86);
    }

    #[test]
    fn test_editor_paste_at_measure_start() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['9', '9', '9']);
        
        ed.copy_columns(0, 2);
        
        ed.cursor.col = 17; // Start of M2
        ed.paste_columns();
        
        // M2: clip (3) + 13 empty of M2 = 16 cols
        assert_eq!(ed.document.columns[17].get_char(0), '9');
        assert_eq!(ed.document.columns[18].get_char(0), '9');
        assert_eq!(ed.document.columns[19].get_char(0), '9');
        assert_eq!(ed.document.columns[33].get_char(0), '|');
        
        // M3: 3 empty (overflow M2) + 13 empty M3 = 16 cols
        assert_eq!(ed.document.columns[34].get_char(0), '-');
        assert_eq!(ed.document.columns[50].get_char(0), '|');
        
        assert_eq!(ed.document.columns.len(), 86);
    }

    #[test]
    fn test_editor_clear_box() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['9', '9', '9']);
        assert_eq!(ed.document.columns[0].get_char(0), '9');
        
        ed.clear_box();
        assert_eq!(ed.document.columns[0].get_char(0), '-');
        assert_eq!(ed.document.columns[1].get_char(0), '9'); // Col 1 is not cleared (Box 1)
        assert_eq!(ed.document.columns[2].get_char(0), '9'); // Col 2 is not cleared (Box 2)
        assert_eq!(ed.document.columns.len(), 69);
    }

    #[test]
    fn test_editor_expand_shrink_box() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        
        // Default size is 1
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 1);
        
        ed.expand_active_box();
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 2);
        assert_eq!(ed.document.columns.len(), 70);
        
        ed.shrink_active_box();
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 1);
        assert_eq!(ed.document.columns.len(), 69);
        
        ed.shrink_active_box();
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 1);
    }

    #[test]
    fn test_editor_insert_char() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        
        // Insert '1' at col 0 (default size 1)
        ed.insert_char_in_box('1').unwrap();
        // Columns: [1 (start=true)] [empty (start=false)] ...
        // Cursor should move to 1.
        assert_eq!(ed.cursor.col, 1);
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.document.columns[0].is_box_start, true);
        assert_eq!(ed.document.columns[1].get_char(0), '-');
        assert_eq!(ed.document.columns[1].is_box_start, false);
        
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 2); // Box 0 is now size 2 (cols 0, 1)
        
        // Insert '2' at col 1
        ed.insert_char_in_box('2').unwrap();
        // Columns: [1 (start=true)] [2 (start=false)] [empty (start=false)] ...
        // Cursor should move to 2.
        assert_eq!(ed.cursor.col, 2);
        assert_eq!(ed.document.columns[1].get_char(0), '2');
        assert_eq!(ed.document.columns[2].get_char(0), '-');
        
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 3); // Box 0 is now size 3 (cols 0, 1, 2)
        
        // Delete char before cursor (at col 2, so deletes col 1 ('2'))
        ed.delete_char_before_cursor();
        assert_eq!(ed.cursor.col, 1);
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.document.columns[1].get_char(0), '-'); // '2' is gone
        
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 2); // Box 0 is now size 2
    }
}
