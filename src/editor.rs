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



    pub fn shrink_box_to_fit(&mut self, col: usize) -> usize {
        let num_strings = self.document.tuning.len();
        if col >= self.document.columns.len() { return 0; }
        if self.document.columns[col].is_barline(num_strings) {
            return 0;
        }
        
        let (box_start, box_end) = self.document.box_range(col);
        let mut max_len = 0;
        for string in 0..num_strings {
            let mut len = 0;
            for (i, c) in (box_start..box_end).enumerate() {
                if self.document.columns[c].get_char(string) != '-' {
                    len = i + 1;
                }
            }
            if len > max_len {
                max_len = len;
            }
        }
        
        let target_size = max_len.max(1);
        let current_size = box_end - box_start;
        if target_size < current_size {
            self.save_state();
            let remove_start = box_start + target_size;
            self.document.columns.drain(remove_start..box_end);
            
            if self.cursor.col >= remove_start && self.cursor.col < box_end {
                self.cursor.col = remove_start - 1;
            }
            return box_end - remove_start;
        }
        0
    }

    pub fn jump_to_col(&mut self, new_col: usize) {
        let old_col = self.cursor.col;
        if old_col != new_col {
            let (old_start, old_end) = self.document.box_range(old_col);
            let (new_start, _) = self.document.box_range(new_col);
            if old_start != new_start {
                let removed = self.shrink_box_to_fit(old_col);
                let mut adjusted_new_col = new_col;
                if removed > 0 && new_col >= old_end {
                    adjusted_new_col -= removed;
                }
                self.cursor.col = adjusted_new_col.min(self.document.columns.len().saturating_sub(1));
            } else {
                self.cursor.col = new_col;
            }
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

        self.jump_to_col(new_col);
        self.cursor.string = new_string;
    }

    pub fn jump_to_measure(&mut self, target_measure: usize) {
        let mut target = self.document.columns.len().saturating_sub(1);
        for i in 0..self.document.columns.len() {
            if self.document.is_measure_start(i) && self.document.measure_number_at_col(i) == target_measure {
                target = i;
                break;
            }
        }
        self.jump_to_col(target);
    }

    pub fn jump_next_measure(&mut self) {
        let num_strings = self.document.tuning.len();
        let mut target = self.document.columns.len().saturating_sub(1);
        for i in (self.cursor.col + 1)..self.document.columns.len() {
            if self.document.columns[i].is_barline(num_strings) {
                target = i + 1;
                break;
            }
        }
        if target >= self.document.columns.len() {
            self.document.append_measure();
        }
        self.jump_to_col(target);
    }

    pub fn jump_prev_measure(&mut self) {
        if self.cursor.col == 0 { return; }
        let mut search_start = self.cursor.col.saturating_sub(1);
        if self.document.columns[search_start].is_barline(self.document.tuning.len()) {
            search_start = search_start.saturating_sub(1);
        }

        let mut target = 0;
        for i in (0..=search_start).rev() {
            if self.document.columns[i].is_barline(self.document.tuning.len()) {
                target = i + 1;
                break;
            }
        }
        self.jump_to_col(target);
    }

    pub fn jump_end_measure(&mut self) {
        let mut start_search = self.cursor.col + 1;
        if start_search < self.document.columns.len() && self.document.columns[start_search].is_barline(self.document.tuning.len()) {
            start_search += 1;
        }

        let mut target = self.document.columns.len().saturating_sub(1);
        for i in start_search..self.document.columns.len() {
            if self.document.columns[i].is_barline(self.document.tuning.len()) {
                target = i.saturating_sub(1);
                break;
            }
        }
        self.jump_to_col(target);
    }

    pub fn jump_next_row(&mut self, wrap_width: usize) {
        let chunks = self.document.calculate_chunks(wrap_width);
        let mut target = self.cursor.col;
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.contains(&self.cursor.col) {
                if i + 1 < chunks.len() {
                    let offset = self.cursor.col - chunk.start;
                    let next_chunk = &chunks[i + 1];
                    target = (next_chunk.start + offset).min(next_chunk.end.saturating_sub(1));
                }
                break;
            }
        }
        self.jump_to_col(target);
    }

    pub fn jump_prev_row(&mut self, wrap_width: usize) {
        let chunks = self.document.calculate_chunks(wrap_width);
        let mut target = self.cursor.col;
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.contains(&self.cursor.col) {
                if i > 0 {
                    let offset = self.cursor.col - chunk.start;
                    let prev_chunk = &chunks[i - 1];
                    target = (prev_chunk.start + offset).min(prev_chunk.end.saturating_sub(1));
                }
                break;
            }
        }
        self.jump_to_col(target);
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
        let string = self.cursor.string;
        let num_strings = self.document.tuning.len();
        if self.document.columns[col].is_barline(num_strings) {
            return;
        }
        self.save_state();
        let (box_start, box_end) = self.document.box_range(col);
        for c in &mut self.document.columns[box_start..box_end] {
            c.set_char(string, '-');
        }
        self.shrink_box_to_fit(col);
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
        let string = self.cursor.string;
        let num_strings = self.document.tuning.len();
        if col < self.document.columns.len() && self.document.columns[col].is_barline(num_strings) {
            return Err("Cannot insert inside a barline");
        }
        
        self.save_state();
        
        if col >= self.document.columns.len() {
            self.document.append_measure();
        }
        
        // If the cursor is on a '-', replace it instead of inserting
        if self.document.columns[col].get_char(string) == '-' {
            self.document.columns[col].set_char(string, c);
            self.cursor.col += 1;
            return Ok(());
        }
        
        let was_start = self.document.columns[col].is_box_start;
        self.document.columns[col].is_box_start = false;
        
        let mut new_col = TabColumn::new();
        new_col.is_box_start = was_start;
        new_col.set_char(string, c);
        
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

    pub fn delete_char_in_box_at_cursor(&mut self) {
        let col = self.cursor.col;
        let string = self.cursor.string;
        let num_strings = self.document.tuning.len();
        if col >= self.document.columns.len() { return; }
        if self.document.columns[col].is_barline(num_strings) {
            return;
        }
        
        self.save_state();
        let (_, box_end) = self.document.box_range(col);
        
        for c in col..(box_end - 1) {
            let next_char = self.document.columns[c + 1].get_char(string);
            self.document.columns[c].set_char(string, next_char);
        }
        self.document.columns[box_end - 1].set_char(string, '-');
        self.shrink_box_to_fit(col);
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
        
        ed.cursor.string = 1;
        ed.replace_chars(&['8', '8', '8']);
        
        assert_eq!(ed.document.columns[0].get_char(0), '9');
        assert_eq!(ed.document.columns[0].get_char(1), '8');
        
        ed.cursor.string = 0;
        ed.clear_box();
        
        assert_eq!(ed.document.columns[0].get_char(0), '-');
        assert_eq!(ed.document.columns[0].get_char(1), '8');
        
        assert_eq!(ed.document.columns[1].get_char(0), '9');
        assert_eq!(ed.document.columns[2].get_char(0), '9');
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
        
        // Initial: [ - ] (size 1)
        // Insert '1' -> replaces '-'
        ed.insert_char_in_box('1').unwrap();
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.cursor.col, 1);
        
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 1); // Box 0 is still size 1
        
        // Now cursor is at 1. Col 1 is '-'
        // Insert '2' -> replaces '-'
        ed.insert_char_in_box('2').unwrap();
        assert_eq!(ed.document.columns[1].get_char(0), '2');
        assert_eq!(ed.cursor.col, 2);
        
        // Now test insertion (not replacement)
        // Go back to 1 (which has '2')
        ed.cursor.col = 1;
        ed.insert_char_in_box('3').unwrap();
        
        // Columns should be: Col 0: '1', Col 1: '3', Col 2: '2'
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.document.columns[1].get_char(0), '3');
        assert_eq!(ed.document.columns[2].get_char(0), '2');
        assert_eq!(ed.cursor.col, 2);
        
        let (s, e) = ed.document.box_range(1);
        assert_eq!(s, 1);
        assert_eq!(e, 3); // Box 1 expanded to size 2 (cols 1, 2)
    }

    #[test]
    fn test_editor_delete_char_in_box() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        
        ed.expand_active_box();
        ed.expand_active_box();
        
        ed.replace_chars(&['1', '2', '3']);
        
        ed.cursor.string = 1;
        ed.replace_chars(&['a', 'b', 'c']);
        
        ed.cursor.string = 0;
        ed.cursor.col = 1;
        ed.delete_char_in_box_at_cursor();
        
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.document.columns[1].get_char(0), '3');
        assert_eq!(ed.document.columns[2].get_char(0), '-');
        
        assert_eq!(ed.document.columns[0].get_char(1), 'a');
        assert_eq!(ed.document.columns[1].get_char(1), 'b');
        assert_eq!(ed.document.columns[2].get_char(1), 'c');
        
        assert_eq!(ed.document.columns.len(), 71);
    }

    #[test]
    fn test_editor_shrink_to_fit() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        
        ed.expand_active_box();
        ed.expand_active_box();
        ed.expand_active_box();
        
        ed.replace_chars(&['1', '2']);
        
        ed.shrink_box_to_fit(0);
        
        let (s, e) = ed.document.box_range(0);
        assert_eq!(e - s, 2);
        assert_eq!(ed.document.columns[0].get_char(0), '1');
        assert_eq!(ed.document.columns[1].get_char(0), '2');
        
        ed.expand_active_box();
        ed.expand_active_box();
        
        ed.jump_to_col(4);
        assert_eq!(ed.cursor.col, 2);
    }
}
