use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;
use crate::editor::Editor;

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Replace { buffer: String },
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
}

impl App {
    pub fn new(editor: Editor, filename: String) -> Self {
        let saved_undo_len = editor.undo_stack.len();
        Self {
            editor,
            mode: Mode::Normal,
            note_mode: false,
            error_msg: None,
            filename,
            saved_undo_len,
            should_quit: false,
            count_buffer: String::new(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.editor.undo_stack.len() != self.saved_undo_len
    }

    fn save_file(&mut self) -> bool {
        match serde_json::to_string(&self.editor) {
            Ok(json) => match std::fs::write(&self.filename, json) {
                Ok(_) => {
                    self.saved_undo_len = self.editor.undo_stack.len();
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
                    self.error_msg = None; // Clear error on next keypress
                    match &mut self.mode {
                        Mode::Normal => self.handle_normal(key),
                        Mode::Replace { buffer } => {
                            let buf = buffer.clone();
                            self.handle_replace(key, buf);
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
            }
        }
        Ok(())
    }

    fn handle_normal(&mut self, key: event::KeyEvent) {
        match key.code {
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
                    KeyCode::Char('j') => self.editor.move_cursor(0, count as isize),
                    KeyCode::Char('k') => self.editor.move_cursor(0, -(count as isize)),
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
                // Check if Ctrl+R for redo
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.editor.redo();
                } else {
                    self.mode = Mode::Replace { buffer: String::new() };
                }
            }
            KeyCode::Char('A') => {
                self.count_buffer.clear();
                self.mode = Mode::Prompt { buffer: String::new() };
            }
            KeyCode::Char('n') => {
                self.count_buffer.clear();
                self.note_mode = !self.note_mode;
            }
            KeyCode::Char('?') => {
                self.count_buffer.clear();
                self.mode = Mode::Help;
            }
            KeyCode::Char('>') => {
                self.count_buffer.clear();
                self.editor.insert_column();
            }
            KeyCode::Char('<') => {
                self.count_buffer.clear();
                self.editor.delete_column();
            }
            KeyCode::Char('u') => {
                self.count_buffer.clear();
                self.editor.undo();
            }
            KeyCode::Char('v') => {
                self.count_buffer.clear();
                self.mode = Mode::Visual { start_col: self.editor.cursor.col };
            }
            KeyCode::Char('p') => {
                self.count_buffer.clear();
                self.editor.paste_columns();
            }
            KeyCode::Char('q') => self.should_quit = true,
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
            KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Char('j') | KeyCode::Char('k') if !buffer.is_empty() => {
                self.commit_replace(&buffer);
                match key.code {
                    KeyCode::Char('h') => self.editor.move_cursor(-1, 0),
                    KeyCode::Char('l') => self.editor.move_cursor(1, 0),
                    KeyCode::Char('j') => self.editor.move_cursor(0, 1),
                    KeyCode::Char('k') => self.editor.move_cursor(0, -1),
                    _ => {}
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('|') => {
                if buffer.is_empty() {
                    if let Err(e) = self.editor.insert_barline() {
                        self.error_msg = Some(e.to_string());
                    }
                    self.mode = Mode::Normal;
                } else {
                    // Invalid, commit whatever is in buffer and return to normal
                    self.commit_replace(&buffer);
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(c) => {
                let is_valid_char = c.is_ascii_digit() || "hpsxbr~t/\\-".contains(c);
                if !is_valid_char {
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

    fn commit_replace(&mut self, buffer: &str) {
        if buffer.is_empty() { return; }
        let chars: Vec<char> = buffer.chars().collect();
        self.editor.replace_chars(&chars);
    }

    fn handle_prompt(&mut self, key: event::KeyEvent, mut buffer: String) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.editor.set_annotation(buffer);
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
            KeyCode::Char('h') => self.editor.move_cursor(-1, 0),
            KeyCode::Char('l') => self.editor.move_cursor(1, 0),
            KeyCode::Char('y') => {
                self.editor.copy_columns(start_col, self.editor.cursor.col);
                self.mode = Mode::Normal;
            }
            KeyCode::Char('x') | KeyCode::Char('d') => {
                self.editor.copy_columns(start_col, self.editor.cursor.col);
                self.editor.delete_columns_range(start_col, self.editor.cursor.col);
                self.mode = Mode::Normal;
            }
            _ => {}
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
                } else if let Ok(col) = cmd.parse::<usize>() {
                    self.editor.jump_to_col(col);
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
