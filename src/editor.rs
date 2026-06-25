use crate::models::document::{TabDocument, Cursor};
use serde::{Serialize, Deserialize};
use crate::models::operation::{Operation, VersionController};
use crate::models::measure::VirtualColumn;


#[derive(Serialize, Deserialize)]
pub struct Editor {
    pub document: TabDocument,
    pub cursor: Cursor,
    pub version_controller: VersionController,
}

impl Editor {
    pub fn new(tuning: Vec<char>) -> Self {
        Self {
            document: TabDocument::new(tuning),
            cursor: Cursor::new(),
            version_controller: VersionController::new(),
        }
    }



    pub fn undo(&mut self) -> Option<String> {
        self.version_controller.undo(&mut self.document)
    }

    pub fn redo(&mut self) -> Option<String> {
        self.version_controller.redo(&mut self.document)
    }

    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        let new_col = (self.cursor.col as isize + dx).max(0) as usize;
        let new_string = (self.cursor.string as isize + dy).clamp(0, self.document.tuning.len().saturating_sub(1) as isize) as usize;
        
        // Ensure the document expands if we move past the end
        while new_col >= self.document.total_global_cols() {
            self.document.append_col();
        }

        self.cursor.col = new_col;
        self.cursor.string = new_string;
    }

    pub fn jump_to_measure(&mut self, target_measure: usize) {
        for i in 0..self.document.total_global_cols() {
            if self.document.is_measure_start(i) && self.document.measure_number_at_col(i) == target_measure {
                self.cursor.col = i;
                return;
            }
        }
        self.cursor.col = self.document.total_global_cols().saturating_sub(1);
    }

    pub fn jump_next_measure(&mut self) {
        for i in (self.cursor.col + 1)..self.document.total_global_cols() {
            if self.document.is_barline(i) {
                self.cursor.col = i + 1;
                if self.cursor.col >= self.document.total_global_cols() {
                    self.document.append_col();
                }
                return;
            }
        }
        self.cursor.col = self.document.total_global_cols().saturating_sub(1);
    }

    pub fn jump_prev_measure(&mut self) {
        if self.cursor.col == 0 { return; }
        let mut search_start = self.cursor.col.saturating_sub(1);
        
        // If we are already at the start of a measure (column right after a barline),
        // skip this barline to jump to the start of the previous measure.
        if self.document.is_barline(search_start) {
            search_start = search_start.saturating_sub(1);
        }

        for i in (0..=search_start).rev() {
            if self.document.is_barline(i) {
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
        if start_search < self.document.total_global_cols() && self.document.is_barline(start_search) {
            start_search += 1;
        }

        for i in start_search..self.document.total_global_cols() {
            if self.document.is_barline(i) {
                self.cursor.col = i.saturating_sub(1);
                return;
            }
        }
        self.cursor.col = self.document.total_global_cols().saturating_sub(1);
    }

    pub fn jump_next_row(&mut self, wrap_width: usize) -> bool {
        let chunks = self.document.calculate_chunks(wrap_width);
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.contains(&self.cursor.col) {
                if i + 1 < chunks.len() {
                    let offset = self.cursor.col - chunk.start;
                    let next_chunk = &chunks[i + 1];
                    self.cursor.col = (next_chunk.start + offset).min(next_chunk.end.saturating_sub(1));
                    return true;
                }
                break;
            }
        }
        false
    }

    pub fn jump_prev_row(&mut self, wrap_width: usize) -> bool {
        let chunks = self.document.calculate_chunks(wrap_width);
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.contains(&self.cursor.col) {
                if i > 0 {
                    let offset = self.cursor.col - chunk.start;
                    let prev_chunk = &chunks[i - 1];
                    self.cursor.col = (prev_chunk.start + offset).min(prev_chunk.end.saturating_sub(1));
                    return true;
                }
                break;
            }
        }
        false
    }

    pub fn insert_column(&mut self) {
        let tuning_len = self.document.tuning.len();
        self.version_controller.apply(&mut self.document, Operation::InsertColumn { 
            col: self.cursor.col, 
            content: VirtualColumn { strings: vec!['-'; tuning_len], annotation: None, is_box_start: false, is_barline: false } 
        });
        // App handles commit
    }

    pub fn delete_char(&mut self) {
        if self.document.is_barline(self.cursor.col) { return; }
        let old_c = self.document.get_char(self.cursor.col, self.cursor.string);
        self.version_controller.apply(&mut self.document, Operation::DeleteChar { 
            col: self.cursor.col, 
            string: self.cursor.string, 
            old_c 
        });
        // App handles commit
    }

    pub fn delete_column(&mut self) {
        if self.document.total_global_cols() == 0  { return; }
        let content = self.document.get_virtual_column(self.cursor.col);
        self.version_controller.apply(&mut self.document, Operation::DeleteColumn { 
            col: self.cursor.col, 
            content 
        });
        
        // Ensure there's always at least one column
        if self.document.total_global_cols() == 0  {
            let tuning_len = self.document.tuning.len();
            self.version_controller.apply(&mut self.document, Operation::InsertColumn { 
                col: 0, 
                content: VirtualColumn { strings: vec!['-'; tuning_len], annotation: None, is_box_start: false, is_barline: false } 
            });
        }
        // App handles commit

        if self.cursor.col >= self.document.total_global_cols() {
            self.cursor.col = self.document.total_global_cols() - 1;
        }
    }

    pub fn replace_chars(&mut self, chars: &[char]) {
        let mut old_chars = Vec::new();
        for i in 0..chars.len() {
            if self.cursor.col + i < self.document.total_global_cols() {
                old_chars.push(self.document.get_char(self.cursor.col + i, self.cursor.string));
            } else {
                old_chars.push('-');
                let tuning_len = self.document.tuning.len();
                self.version_controller.apply(&mut self.document, Operation::InsertColumn { 
                    col: self.cursor.col + i, 
                    content: VirtualColumn { strings: vec!['-'; tuning_len], annotation: None, is_box_start: false, is_barline: false } 
                });
            }
        }
        
        self.version_controller.apply(&mut self.document, Operation::ReplaceChars {
            start_col: self.cursor.col,
            string: self.cursor.string,
            old_chars,
            new_chars: chars.to_vec(),
        });
        // App handles commit
    }

    pub fn insert_barline(&mut self) -> Result<(), &'static str> {
        let is_blank = self.document.is_blank(self.cursor.col);
        if !is_blank {
            return Err("Column must be completely blank to insert a barline");
        }

        self.version_controller.apply(&mut self.document, Operation::InsertBarline { col: self.cursor.col });
        // App handles commit
        Ok(())
    }

    pub fn set_annotation(&mut self, text: String) {
        let old_ann = self.document.get_virtual_column(self.cursor.col).annotation;
        self.version_controller.apply(&mut self.document, Operation::SetAnnotation {
            col: self.cursor.col,
            old_ann,
            new_ann: if text.is_empty() { None } else { Some(text) },
        });
        // App handles commit
    }

    pub fn copy_columns(&mut self, start: usize, end: usize) {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let e = e.min(self.document.total_global_cols().saturating_sub(1));
        self.document.copy_cols(s, e);
    }

    pub fn delete_columns_range(&mut self, start: usize, end: usize) {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let e = e.min(self.document.total_global_cols().saturating_sub(1));
        
        let mut contents = Vec::new();
        for i in s..=e {
            contents.push(self.document.get_virtual_column(i));
        }

        self.version_controller.apply(&mut self.document, Operation::DeleteColumnsRange {
            start: s,
            end: e,
            contents,
        });
        
        if self.document.total_global_cols() == 0  {
            let tuning_len = self.document.tuning.len();
            self.version_controller.apply(&mut self.document, Operation::InsertColumn { 
                col: 0, 
                content: VirtualColumn { strings: vec!['-'; tuning_len], annotation: None, is_box_start: false, is_barline: false } 
            });
        }
        // App handles commit
        
        self.cursor.col = s.min(self.document.total_global_cols().saturating_sub(1));
    }

    pub fn paste_columns(&mut self) {
        if self.document.clipboard.is_empty() {
            return;
        }
        
        let mut contents = Vec::new();
        for m in &self.document.clipboard {
            let mut strings = Vec::new();
            for s in 0..self.document.tuning.len() {
                strings.push(m.get_char(s, 0));
            }
            contents.push(VirtualColumn {
                strings,
                annotation: m.annotations.get(&0).cloned(),
                is_box_start: m.box_starts.contains(&0),
                is_barline: false,
            });
        }
        self.version_controller.apply(&mut self.document, Operation::InsertColumnsRange {
            start: self.cursor.col,
            contents,
        });
        // App handles commit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_insert_delete_column() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        assert_eq!(ed.document.total_global_cols(), 64); // 4 measures * 15 cols + 5 barlines = 65
        ed.insert_column();
        ed.version_controller.commit("Insert");
        assert_eq!(ed.document.total_global_cols(), 65);
        ed.undo();
        assert_eq!(ed.document.total_global_cols(), 64);
        ed.redo();
        assert_eq!(ed.document.total_global_cols(), 65);

        ed.delete_column();
        ed.version_controller.commit("Delete");
        assert_eq!(ed.document.total_global_cols(), 64);
    }

    #[test]
    fn test_editor_replace_chars() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['1', '1']);
        ed.version_controller.commit("Replace");
        assert_eq!(ed.document.get_char(0, 0), '1');
        assert_eq!(ed.document.get_char(1, 0), '1');
        assert_eq!(ed.document.get_char(2, 0), '-');

        ed.undo();
        assert_eq!(ed.document.get_char(0, 0), '-');
        assert_eq!(ed.document.get_char(1, 0), '-');
    }

    #[test]
    fn test_editor_copy_paste() {
        let mut ed = Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']);
        ed.cursor.col = 0;
        ed.cursor.string = 0;
        ed.replace_chars(&['9']);
        ed.version_controller.commit("Replace");
        
        ed.copy_columns(0, 0);
        assert_eq!(ed.document.clipboard.len(), 1);
        assert_eq!(ed.document.clipboard[0].get_char(0, 0), '9');

        ed.cursor.col = 5;
        ed.paste_columns();
        ed.version_controller.commit("Paste");
        assert_eq!(ed.document.get_char(5, 0), '9');
        assert_eq!(ed.document.total_global_cols(), 65);
    }
}
