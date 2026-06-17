use crate::document::{TabColumn, TabDocument, Cursor};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Editor {
    pub document: TabDocument,
    pub cursor: Cursor,
    pub undo_stack: Vec<(TabDocument, Cursor)>,
    pub redo_stack: Vec<(TabDocument, Cursor)>,
}

impl Editor {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            document: TabDocument::new(),
            cursor: Cursor::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn new_with_measures() -> Self {
        let mut ed = Self {
            document: TabDocument { columns: Vec::new(), tuning: ['e', 'B', 'G', 'D', 'A', 'E'], clipboard: Vec::new() },
            cursor: Cursor::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        for i in 0..4 {
            for _ in 0..15 {
                ed.document.columns.push(TabColumn::new());
            }
            if i < 3 {
                let mut bar = TabColumn::new();
                bar.strings = ['|'; 6];
                ed.document.columns.push(bar);
            }
        }
        ed
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
        let new_col = (self.cursor.col as isize + dx).max(0) as usize;
        let new_string = (self.cursor.string as isize + dy).clamp(0, 5) as usize;
        
        // Ensure the document expands if we move past the end
        while new_col >= self.document.columns.len() {
            self.document.columns.push(TabColumn::new());
        }

        self.cursor.col = new_col;
        self.cursor.string = new_string;
    }

    pub fn jump_to_col(&mut self, col: usize) {
        while self.document.columns.len() <= col {
            self.document.columns.push(TabColumn::new());
        }
        self.cursor.col = col;
    }

    pub fn jump_next_measure(&mut self) {
        for i in (self.cursor.col + 1)..self.document.columns.len() {
            if self.document.columns[i].is_barline() {
                self.cursor.col = i + 1;
                if self.cursor.col >= self.document.columns.len() {
                    self.document.columns.push(TabColumn::new());
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
        if self.document.columns[search_start].is_barline() {
            search_start = search_start.saturating_sub(1);
        }

        for i in (0..=search_start).rev() {
            if self.document.columns[i].is_barline() {
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
        if start_search < self.document.columns.len() && self.document.columns[start_search].is_barline() {
            start_search += 1;
        }

        for i in start_search..self.document.columns.len() {
            if self.document.columns[i].is_barline() {
                self.cursor.col = i.saturating_sub(1);
                return;
            }
        }
        self.cursor.col = self.document.columns.len().saturating_sub(1);
    }

    pub fn insert_column(&mut self) {
        self.save_state();
        self.document.columns.insert(self.cursor.col, TabColumn::new());
    }

    pub fn delete_column(&mut self) {
        if self.document.columns.is_empty() { return; }
        self.save_state();
        self.document.columns.remove(self.cursor.col);
        // Ensure there's always at least one column
        if self.document.columns.is_empty() {
            self.document.columns.push(TabColumn::new());
        }
        if self.cursor.col >= self.document.columns.len() {
            self.cursor.col = self.document.columns.len() - 1;
        }
    }

    pub fn replace_chars(&mut self, chars: &[char]) {
        self.save_state();
        
        // Ensure we have enough columns to fit the characters
        while self.cursor.col + chars.len() > self.document.columns.len() {
            self.document.columns.push(TabColumn::new());
        }

        for (i, &c) in chars.iter().enumerate() {
            self.document.columns[self.cursor.col + i].strings[self.cursor.string] = c;
        }
    }

    pub fn insert_barline(&mut self) -> Result<(), &'static str> {
        let is_blank = self.document.columns[self.cursor.col].is_blank();
        if !is_blank {
            return Err("Column must be completely blank to insert a barline");
        }

        self.save_state();
        self.document.columns[self.cursor.col] = TabColumn::barline();
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
        
        // We use splice to insert the clipboard at the cursor.
        let tail = self.document.columns.split_off(self.cursor.col);
        self.document.columns.extend(clip);
        self.document.columns.extend(tail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_insert_delete_column() {
        let mut ed = Editor::new();
        assert_eq!(ed.document.columns.len(), 80);
        ed.insert_column();
        assert_eq!(ed.document.columns.len(), 81);
        ed.undo();
        assert_eq!(ed.document.columns.len(), 80);
        ed.redo();
        assert_eq!(ed.document.columns.len(), 81);

        ed.delete_column();
        assert_eq!(ed.document.columns.len(), 80);
    }

    #[test]
    fn test_editor_replace_chars() {
        let mut ed = Editor::new();
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['1', '1']);
        assert_eq!(ed.document.columns[0].strings[0], '1');
        assert_eq!(ed.document.columns[1].strings[0], '1');
        assert_eq!(ed.document.columns[2].strings[0], '-');

        ed.undo();
        assert_eq!(ed.document.columns[0].strings[0], '-');
        assert_eq!(ed.document.columns[1].strings[0], '-');
    }

    #[test]
    fn test_editor_copy_paste() {
        let mut ed = Editor::new();
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['9']);
        
        ed.copy_columns(0, 0);
        assert_eq!(ed.document.clipboard.len(), 1);
        assert_eq!(ed.document.clipboard[0].strings[0], '9');

        ed.cursor.col = 5;
        ed.paste_columns();
        assert_eq!(ed.document.columns[5].strings[0], '9');
        assert_eq!(ed.document.columns.len(), 81); // 80 original + 1 pasted
    }
}
