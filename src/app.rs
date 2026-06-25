use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;
use crate::editor::Editor;

fn is_valid_replace_char(c: char) -> bool {
    c.is_ascii_digit() || "hpsxbr~t/\\-".contains(c)
}

fn is_valid_continuous_replace_char(c: char) -> bool {
    c.is_ascii_digit() || "hpsxbr~t/\\- |".contains(c)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Replace { buffer: String },
    ContinuousReplace { start_col: usize },
    Insert { start_col: usize },
    Prompt { buffer: String },
    Visual { start_col: usize },
    Command { buffer: String },
    Help,
}

pub struct App {
    pub editor: Editor,
    pub mode: Mode,
    pub note_mode: bool,
    pub error_msg: Option<String>,
    pub filename: String,
    pub saved_undo_len: usize,
    pub should_quit: bool,
    pub count_buffer: String,
    pub key_log: String,
}

impl App {
    pub fn new(editor: Editor, filename: String) -> Self {
        let saved_undo_len = editor.version_controller.undo_stack.len();
        Self {
            editor,
            mode: Mode::Normal,
            note_mode: false,
            error_msg: None,
            filename,
            saved_undo_len,
            should_quit: false,
            count_buffer: String::new(),
            key_log: String::new(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.editor.version_controller.undo_stack.len() != self.saved_undo_len
    }


    fn move_cursor_vertical(&mut self, delta: isize, count: usize) {
        let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
        let wrap_width = if width > 4 { (width - 4) as usize } else { 80 };
        let num_strings = self.editor.document.tuning.len();

        for _ in 0..count {
            if delta > 0 {
                if self.editor.cursor.string + 1 < num_strings {
                    self.editor.move_cursor(0, 1);
                } else if self.editor.jump_next_row(wrap_width) {
                    self.editor.cursor.string = 0;
                }
            } else if delta < 0 {
                if self.editor.cursor.string > 0 {
                    self.editor.move_cursor(0, -1);
                } else if self.editor.jump_prev_row(wrap_width) {
                    self.editor.cursor.string = num_strings - 1;
                }
            }
        }
    }

    fn save_file(&mut self) -> bool {
        match serde_json::to_string(&self.editor) {
            Ok(json) => match std::fs::write(&self.filename, json) {
                Ok(_) => {
                    self.saved_undo_len = self.editor.version_controller.undo_stack.len();
                    self.error_msg = Some(format!("Saved {}", self.filename));
                    true
                }
                Err(e) => {
                    self.error_msg = Some(format!("Error saving file: {}", e));
                    false
                }
            },
            Err(e) => {
                self.error_msg = Some(format!("Serialization error: {}", e));
                false
            }
        }
    }

    pub fn handle_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key(key);
                }
            }
        }
        Ok(())
    }

    pub fn handle_key(&mut self, key: event::KeyEvent) {
        let key_str = match key.code {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Esc => "<Esc>".to_string(),
            KeyCode::Enter => "<Enter>".to_string(),
            KeyCode::Backspace => "<Backspace>".to_string(),
            KeyCode::Tab => "<Tab>".to_string(),
            KeyCode::Left => "<Left>".to_string(),
            KeyCode::Right => "<Right>".to_string(),
            KeyCode::Up => "<Up>".to_string(),
            KeyCode::Down => "<Down>".to_string(),
            _ => format!("{:?}", key.code),
        };
        self.key_log.push_str(&key_str);
        if self.key_log.len() > 60 {
            let remove_len = self.key_log.len() - 60;
            self.key_log.drain(..remove_len);
        }

        self.error_msg = None; // Clear error on next keypress
        match &mut self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Replace { buffer } => {
                let buf = buffer.clone();
                self.handle_replace(key, buf);
            }
            Mode::ContinuousReplace { start_col } => {
                let sc = *start_col;
                self.handle_continuous_replace(key, sc);
            }
            Mode::Prompt { buffer } => {
                let buf = buffer.clone();
                self.handle_prompt(key, buf);
            }
            Mode::Visual { start_col } => {
                let start = *start_col;
                self.handle_visual(key, start);
            }
            Mode::Command { buffer } => {
                let buf = buffer.clone();
                self.handle_command(key, buf);
            }
            Mode::Insert { start_col } => {
                let sc = *start_col;
                self.handle_insert(key, sc);
            }
            Mode::Help => {
                match key.code {
                    event::KeyCode::Esc | event::KeyCode::Char('?') | event::KeyCode::Char('q') => {
                        self.mode = Mode::Normal;
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_normal(&mut self, key: event::KeyEvent) {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.count_buffer.push(c);
            }
            KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('w') | KeyCode::Char('e') | KeyCode::Char('b') | KeyCode::Char('[') | KeyCode::Char(']') | KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                let count = if self.count_buffer.is_empty() {
                    1
                } else {
                    self.count_buffer.parse::<usize>().unwrap_or(1).max(1)
                };
                self.count_buffer.clear();

                match key.code {
                    KeyCode::Char('h') | KeyCode::Left => self.editor.move_cursor(-(count as isize), 0),
                    KeyCode::Char('l') | KeyCode::Right => self.editor.move_cursor(count as isize, 0),
                    KeyCode::Char('j') | KeyCode::Down => self.move_cursor_vertical(1, count),
                    KeyCode::Char('k') | KeyCode::Up => self.move_cursor_vertical(-1, count),
                    KeyCode::Char('w') => { for _ in 0..count { self.editor.jump_next_measure(); } }
                    KeyCode::Char('b') => { for _ in 0..count { self.editor.jump_prev_measure(); } }
                    KeyCode::Char('e') => { for _ in 0..count { self.editor.jump_end_measure(); } }
                    KeyCode::Char(']') => {
                        let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
                        let wrap_width = if width > 4 { (width - 4) as usize } else { 80 };
                        for _ in 0..count { self.editor.jump_next_row(wrap_width); }
                    }
                    KeyCode::Char('[') => {
                        let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
                        let wrap_width = if width > 4 { (width - 4) as usize } else { 80 };
                        for _ in 0..count { self.editor.jump_prev_row(wrap_width); }
                    }
                    _ => {}
                }
            }
            KeyCode::Char(':') => {
                self.count_buffer.clear();
                self.mode = Mode::Command { buffer: String::new() };
            }
            KeyCode::Char('r') => {
                self.count_buffer.clear();
                self.mode = Mode::Replace { buffer: String::new() };
            }
            KeyCode::Char('U') => {
                self.count_buffer.clear();
                // Redo
                if let Some(desc) = self.editor.redo() {
                    self.error_msg = Some(format!("Redid: {}", desc));
                } else {
                    self.error_msg = None;
                }
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.count_buffer.clear();
                self.mode = Mode::Insert { start_col: self.editor.cursor.col };
            }
            KeyCode::Char('R') => {
                self.count_buffer.clear();
                self.mode = Mode::ContinuousReplace { start_col: self.editor.cursor.col };
            }
            KeyCode::Char('A') => {
                self.count_buffer.clear();
                self.mode = Mode::Prompt { buffer: String::new() };
            }
            KeyCode::Char('n') => {
                self.count_buffer.clear();
                self.note_mode = !self.note_mode;
            }
            KeyCode::Char('x') => {
                self.count_buffer.clear();
                self.editor.delete_char();
                self.editor.version_controller.commit("Delete character");
            }
            KeyCode::Char('?') => {
                self.count_buffer.clear();
                self.mode = Mode::Help;
            }
            KeyCode::Char('>') => {
                self.count_buffer.clear();
                self.editor.insert_column();
                self.editor.version_controller.commit("Insert column");
            }
            KeyCode::Char('<') => {
                self.count_buffer.clear();
                self.editor.delete_column();
                self.editor.version_controller.commit("Delete column");
            }
            KeyCode::Char('u') => {
                self.count_buffer.clear();
                // Undo
                if let Some(desc) = self.editor.undo() {
                    self.error_msg = Some(format!("Undid: {}", desc));
                } else {
                    self.error_msg = None;
                }
            }
            KeyCode::Char('v') => {
                self.count_buffer.clear();
                self.mode = Mode::Visual { start_col: self.editor.cursor.col };
            }
            KeyCode::Char('p') => {
                self.count_buffer.clear();
                self.editor.paste_columns();
                self.editor.version_controller.commit("Paste columns");
            }
            _ => {
                // Clear buffer on invalid command
                self.count_buffer.clear();
            }
        }
    }

    fn handle_replace(&mut self, key: event::KeyEvent, mut buffer: String) {
        match key.code {
            KeyCode::Esc => {
                self.commit_replace(&buffer);
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.commit_replace(&buffer);
                let num_strings = self.editor.document.tuning.len();
                if self.editor.cursor.string + 1 < num_strings {
                    self.editor.cursor.string += 1;
                    self.mode = Mode::Replace { buffer: String::new() };
                } else {
                    self.editor.cursor.string = 0;
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Char('j') | KeyCode::Char('k') if !buffer.is_empty() => {
                self.commit_replace(&buffer);
                match key.code {
                    KeyCode::Char('h') => self.editor.move_cursor(-1, 0),
                    KeyCode::Char('l') => self.editor.move_cursor(1, 0),
                    KeyCode::Char('j') => self.move_cursor_vertical(1, 1),
                    KeyCode::Char('k') => self.move_cursor_vertical(-1, 1),
                    _ => {}
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('|') => {
                if buffer.is_empty() {
                    if let Err(e) = self.editor.insert_barline() {
                        self.error_msg = Some(e.to_string());
                    } else {
                        self.editor.version_controller.commit("Insert barline");
                    }
                    self.mode = Mode::Normal;
                } else {
                    // Invalid, commit whatever is in buffer and return to normal
                    self.commit_replace(&buffer);
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(c) => {
                if !is_valid_replace_char(c) {
                    self.error_msg = Some(format!("Invalid character: '{}'", c));
                    self.mode = Mode::Normal;
                    return;
                }

                // Buffer the character
                buffer.push(c);
                
                // If it's not a digit, immediately commit and go back to normal
                if !c.is_ascii_digit() {
                    self.commit_replace(&buffer);
                    self.mode = Mode::Normal;
                } else {
                    // If it is a digit and length is 2, commit automatically
                    if buffer.len() >= 2 {
                        self.commit_replace(&buffer);
                        self.mode = Mode::Normal;
                    } else {
                        // Keep buffering
                        self.mode = Mode::Replace { buffer };
                    }
                }
            }
            _ => {
                self.commit_replace(&buffer);
                self.mode = Mode::Normal;
            }
        }
    }

    fn handle_continuous_replace(&mut self, key: event::KeyEvent, start_col: usize) {
        match key.code {
            KeyCode::Enter => {
                let num_strings = self.editor.document.tuning.len();
                if self.editor.cursor.string + 1 < num_strings {
                    self.editor.version_controller.commit("Continuous replace");
                    self.editor.cursor.string += 1;
                    self.editor.cursor.col = start_col;
                } else {
                    self.editor.cursor.string = 0;
                    self.editor.cursor.col = start_col;
                    self.editor.version_controller.commit("Continuous replace");
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Esc => {
                self.editor.version_controller.commit("Continuous replace");
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                let _tuning_len = self.editor.document.tuning.len();
                
                // Move left by 1 if possible
                if self.editor.cursor.col > 0 {
                    self.editor.move_cursor(-1, 0);
                }
                
                // Keep moving left if we land on a barline
                while self.editor.cursor.col > 0 && self.editor.document.is_barline(self.editor.cursor.col) {
                    self.editor.move_cursor(-1, 0);
                }
                
                // Reset the char to a dash if it's not a barline
                if !self.editor.document.is_barline(self.editor.cursor.col) {
                    self.editor.replace_chars(&['-']);
                }
            }
            KeyCode::Up => {
                self.editor.move_cursor(0, -1);
            }
            KeyCode::Down => {
                self.editor.move_cursor(0, 1);
            }
            KeyCode::Left => {
                let _tuning_len = self.editor.document.tuning.len();
                if self.editor.cursor.col > 0 {
                    self.editor.move_cursor(-1, 0);
                }
                while self.editor.cursor.col > 0 && self.editor.document.is_barline(self.editor.cursor.col) {
                    self.editor.move_cursor(-1, 0);
                }
            }
            KeyCode::Right => {
                let _tuning_len = self.editor.document.tuning.len();
                self.editor.move_cursor(1, 0);
                while self.editor.cursor.col < self.editor.document.total_global_cols() 
                    && self.editor.document.is_barline(self.editor.cursor.col) 
                {
                    self.editor.move_cursor(1, 0);
                }
            }
            KeyCode::Char(c) => {
                if !is_valid_continuous_replace_char(c) {
                    self.error_msg = Some(format!("Invalid character: '{}'", c));
                    self.mode = Mode::Normal;
                    return;
                }

                if c == '|' {
                    if let Err(e) = self.editor.insert_barline() {
                        self.error_msg = Some(e.to_string());
                        return;
                    }
                    self.editor.move_cursor(1, 0);
                } else {
                    let insert_c = if c == ' ' { '-' } else { c };
                    self.editor.replace_chars(&[insert_c]);
                    self.editor.move_cursor(1, 0);
                }

                // Skip any existing barlines so we don't accidentally overwrite them
                let _tuning_len = self.editor.document.tuning.len();
                while self.editor.cursor.col < self.editor.document.total_global_cols() 
                    && self.editor.document.is_barline(self.editor.cursor.col) 
                {
                    self.editor.move_cursor(1, 0);
                }
            }
            _ => {
                self.mode = Mode::Normal;
            }
        }
    }

    
    fn handle_insert(&mut self, key: event::KeyEvent, start_col: usize) {
        match key.code {
            KeyCode::Esc => {
                self.editor.version_controller.commit("Insert");
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let num_strings = self.editor.document.tuning.len();
                if self.editor.cursor.string + 1 < num_strings {
                    self.editor.version_controller.commit("Insert");
                    self.editor.cursor.string += 1;
                    self.editor.cursor.col = start_col;
                    self.mode = Mode::ContinuousReplace { start_col };
                } else {
                    self.editor.cursor.string = 0;
                    self.editor.cursor.col = start_col;
                    self.editor.version_controller.commit("Insert");
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Backspace => {
                if self.editor.cursor.col > 0 {
                    self.editor.move_cursor(-1, 0);
                    self.editor.delete_column();
                }
            }
            KeyCode::Char(c) => {
                if !is_valid_continuous_replace_char(c) {
                    self.error_msg = Some(format!("Invalid character: '{}'", c));
                    self.mode = Mode::Normal;
                    return;
                }
                if c == '|' {
                    if let Err(e) = self.editor.insert_barline() {
                        self.error_msg = Some(e.to_string());
                        return;
                    }
                    self.editor.move_cursor(1, 0);
                } else {
                    let insert_c = if c == ' ' { '-' } else { c };
                    self.editor.insert_column();
                    self.editor.replace_chars(&[insert_c]);
                    self.editor.move_cursor(1, 0);
                }
            }
            _ => {
                self.mode = Mode::Normal;
            }
        }
    }

    fn commit_replace(&mut self, buffer: &str) {
        if buffer.is_empty() { return; }
        let chars: Vec<char> = buffer.chars().collect();
        self.editor.replace_chars(&chars);
        self.editor.version_controller.commit("Replace characters");
    }

    fn handle_prompt(&mut self, key: event::KeyEvent, mut buffer: String) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.editor.set_annotation(buffer);
                self.editor.version_controller.commit("Set annotation");
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                buffer.pop();
                self.mode = Mode::Prompt { buffer };
            }
            KeyCode::Char(c) => {
                buffer.push(c);
                self.mode = Mode::Prompt { buffer };
            }
            _ => {}
        }
    }

    fn handle_visual(&mut self, key: event::KeyEvent, start_col: usize) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.count_buffer.push(c);
            }
            KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('w') | KeyCode::Char('e') | KeyCode::Char('b') | KeyCode::Char('[') | KeyCode::Char(']') => {
                let count = if self.count_buffer.is_empty() {
                    1
                } else {
                    self.count_buffer.parse::<usize>().unwrap_or(1).max(1)
                };
                self.count_buffer.clear();

                match key.code {
                    KeyCode::Char('h') => self.editor.move_cursor(-(count as isize), 0),
                    KeyCode::Char('l') => self.editor.move_cursor(count as isize, 0),
                    KeyCode::Char('j') => self.move_cursor_vertical(1, count),
                    KeyCode::Char('k') => self.move_cursor_vertical(-1, count),
                    KeyCode::Char('w') => { for _ in 0..count { self.editor.jump_next_measure(); } }
                    KeyCode::Char('b') => { for _ in 0..count { self.editor.jump_prev_measure(); } }
                    KeyCode::Char('e') => { for _ in 0..count { self.editor.jump_end_measure(); } }
                    KeyCode::Char(']') => {
                        let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
                        let wrap_width = if width > 4 { (width - 4) as usize } else { 80 };
                        for _ in 0..count { self.editor.jump_next_row(wrap_width); }
                    }
                    KeyCode::Char('[') => {
                        let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
                        let wrap_width = if width > 4 { (width - 4) as usize } else { 80 };
                        for _ in 0..count { self.editor.jump_prev_row(wrap_width); }
                    }
                    _ => {}
                }
            }
            KeyCode::Char('y') => {
                self.count_buffer.clear();
                self.editor.copy_columns(start_col, self.editor.cursor.col);
                self.mode = Mode::Normal;
            }
            KeyCode::Char('x') | KeyCode::Char('d') => {
                self.count_buffer.clear();
                self.editor.copy_columns(start_col, self.editor.cursor.col);
                self.editor.delete_columns_range(start_col, self.editor.cursor.col);
                self.editor.version_controller.commit("Delete columns");
                self.mode = Mode::Normal;
            }
            _ => {
                self.count_buffer.clear();
            }
        }
    }

    fn handle_command(&mut self, key: event::KeyEvent, mut buffer: String) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let cmd = buffer.trim();
                if cmd == "q" {
                    if self.is_dirty() {
                        self.error_msg = Some("Unsaved changes! Use :q! to force quit.".to_string());
                    } else {
                        self.should_quit = true;
                    }
                } else if cmd == "q!" {
                    self.should_quit = true;
                } else if cmd == "w" {
                    self.save_file();
                } else if cmd == "wq" {
                    if self.save_file() {
                        self.should_quit = true;
                    }
                } else if let Ok(measure_num) = cmd.parse::<usize>() {
                    self.editor.jump_to_measure(measure_num);
                } else {
                    self.error_msg = Some("Invalid command".to_string());
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                buffer.pop();
                self.mode = Mode::Command { buffer };
            }
            KeyCode::Char(c) => {
                buffer.push(c);
                self.mode = Mode::Command { buffer };
            }
            _ => {}
        }
    }
}
