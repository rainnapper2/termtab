use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};
use crate::app::{App, Mode};
use crate::notes::{fret_to_note, parse_key_signature};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3), // For status/error
        ])
        .split(f.size());

    let (tab_text, cursor_pos) = render_tab_document(app, chunks[0].width as usize);
    
    let mut scroll_y = 0;
    if let Some((_, cy)) = cursor_pos {
        let row_center_y = cy.saturating_sub(app.editor.cursor.string) + 3;
        let visible_height = chunks[0].height.saturating_sub(2);
        if row_center_y as u16 > visible_height / 2 {
            scroll_y = (row_center_y as u16) - (visible_height / 2);
        }
    }

    let paragraph = Paragraph::new(tab_text)
        .block(Block::default().borders(Borders::ALL).title(" TermTab "))
        .scroll((scroll_y, 0));
    
    f.render_widget(paragraph, chunks[0]);

    if !matches!(app.mode, Mode::Help) {
        if let Some((cx, cy)) = cursor_pos {
            let actual_cy = (cy as u16).saturating_sub(scroll_y);
            if actual_cy < chunks[0].height.saturating_sub(2) {
                f.set_cursor(chunks[0].x + 1 + cx as u16, chunks[0].y + 1 + actual_cy);
            }
        }
    }

    // Status bar
    let mode_str = match &app.mode {
        Mode::Normal => "NORMAL".to_string(),
        Mode::Replace { buffer } => format!("REPLACE [{}]", buffer),
        Mode::Prompt { buffer } => format!("PROMPT [{}]", buffer),
        Mode::Visual { start_col } => format!("VISUAL [start: {}]", start_col),
        Mode::Command { buffer } => format!("COMMAND [:{}]", buffer),
        Mode::Help => "HELP".to_string(),
    };

    let status_style = Style::default().bg(Color::Blue).fg(Color::White);

    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(25)])
        .split(chunks[1]);

    let status_left = Paragraph::new(Line::from(vec![
        Span::styled(format!(" MODE: {} ", mode_str), status_style),
        Span::styled(
            if let Some(msg) = &app.error_msg { format!(" | {} ", msg) } else { " | ".to_string() },
            Style::default().fg(Color::Red),
        ),
    ]))
    .block(Block::default().borders(Borders::TOP));

    let status_right_text = if !app.key_log.is_empty() {
        let max_len = 23;
        if app.key_log.chars().count() > max_len {
            app.key_log.chars().skip(app.key_log.chars().count() - max_len).collect()
        } else {
            app.key_log.clone()
        }
    } else {
        "Type ? for help".to_string()
    };
    let status_right = Paragraph::new(format!("{}  ", status_right_text))
        .alignment(ratatui::layout::Alignment::Right)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(status_left, status_layout[0]);
    f.render_widget(status_right, status_layout[1]);

    if let Mode::Help = app.mode {
        let help_text = vec![
            Line::from(Span::styled(" TermTab Cheatsheet ", Style::default().fg(Color::Cyan))),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  h, j, k, l   Move cursor left, down, up, right"),
            Line::from("  w, e, b      Jump to next, end, or previous measure"),
            Line::from("  5l, 3w       Numeric prefixes to multiply movements"),
            Line::from(""),
            Line::from("Editing:"),
            Line::from("  r            Enter Replace mode (type digits or h,p,s,x,b,r,~,t,/,-)"),
            Line::from("  v            Enter Visual mode (select columns)"),
            Line::from("  y, d, p      Yank (copy), delete (cut), paste selected columns"),
            Line::from("  >, <         Insert or delete column at cursor"),
            Line::from("  A            Add text annotation (e.g., Key: C Major)"),
            Line::from("  n            Toggle diatonic note translation (fret numbers -> notes)"),
            Line::from("  u, Ctrl+R    Undo, Redo"),
            Line::from(""),
            Line::from("Files:"),
            Line::from("  :w, :q, :wq  Save, quit, save & quit"),
            Line::from("  :120         Jump directly to measure 120"),
            Line::from(""),
            Line::from(Span::styled("See the README.md for a full comprehensive guide.", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("Press Esc or ? to close", Style::default().fg(Color::Gray))),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(Block::default().title(" Help ").borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);

        let area = centered_rect(80, 80, f.size());
        f.render_widget(Clear, area);
        f.render_widget(help_paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_tab_document(app: &App, max_width: usize) -> (Text<'static>, Option<(usize, usize)>) {
    let mut lines = Vec::new();
    let wrap_width = if max_width > 4 { max_width - 4 } else { 80 }; // Keep margin for borders
    
    let mut cursor_visual_pos = None;

    // Pre-calculate active key for every column so multiple key changes work properly
    let mut current_key: Option<String> = None;
    let mut column_keys = Vec::with_capacity(app.editor.document.columns.len());
    for col in &app.editor.document.columns {
        if let Some(text) = &col.annotation {
            if let Some(k) = parse_key_signature(text) {
                current_key = Some(k);
            }
        }
        column_keys.push(current_key.clone());
    }

    let chunks = app.editor.document.calculate_chunks(wrap_width);

    for chunk_range in chunks {
        let current_col = chunk_range.start;
        let chunk = &app.editor.document.columns[chunk_range];

        // 1. Process Annotations for this chunk
        // Stack them if they overlap.
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
            let global_col = current_col + i;
            let offset_i = i + 2; // Offset by 2 to align past the "e|" tuning prefix

            if app.editor.document.is_measure_start(global_col) {
                let text = format!("[{}]", app.editor.document.measure_number_at_col(global_col));
                place_text(&text, offset_i, &mut measure_lines);
            }

            if let Some(text) = &col.annotation {
                place_text(text, offset_i, &mut annotation_lines);
            }
        }

        for m_line in measure_lines {
            let s: String = m_line.into_iter().collect();
            lines.push(Line::from(Span::styled(s, Style::default().fg(Color::Cyan))));
        }

        for a_line in annotation_lines {
            let s: String = a_line.into_iter().collect();
            lines.push(Line::from(Span::styled(s, Style::default().fg(Color::Yellow))));
        }

        let start_y = lines.len();

        // 2. Process Strings
        for string_idx in 0..6 {
            let mut string_chars = Vec::new();
            
            // Add tuning letter
            let tuning_char = app.editor.document.tuning[string_idx];
            string_chars.push(Span::styled(format!("{}|", tuning_char), Style::default().fg(Color::DarkGray)));

            let mut i = 0;
            while i < chunk.len() {
                let global_col = current_col + i;
                let c = chunk[i].strings[string_idx];
                
                let mut is_selected = false;
                if let Mode::Visual { start_col } = app.mode {
                    let min_c = start_col.min(app.editor.cursor.col);
                    let max_c = start_col.max(app.editor.cursor.col);
                    if global_col >= min_c && global_col <= max_c {
                        is_selected = true;
                    }
                }

                let style = if is_selected {
                    Style::default().bg(Color::White).fg(Color::Black)
                } else {
                    Style::default()
                };

                if app.note_mode && c.is_ascii_digit() {
                    // Try to parse fret
                    let mut fret_str = c.to_string();
                    let mut consumed_next = false;
                    if i + 1 < chunk.len() && chunk[i+1].strings[string_idx].is_ascii_digit() {
                        fret_str.push(chunk[i+1].strings[string_idx]);
                        consumed_next = true;
                    }
                    
                    if let Ok(fret) = fret_str.parse::<u32>() {
                        let note = match string_idx {
                            0 => fret_to_note('e', fret, column_keys[global_col].as_deref()),
                            1 => fret_to_note('B', fret, column_keys[global_col].as_deref()),
                            2 => fret_to_note('G', fret, column_keys[global_col].as_deref()),
                            3 => fret_to_note('D', fret, column_keys[global_col].as_deref()),
                            4 => fret_to_note('A', fret, column_keys[global_col].as_deref()),
                            5 => fret_to_note('E', fret, column_keys[global_col].as_deref()),
                            _ => fret_to_note('E', fret, column_keys[global_col].as_deref()),
                        };
                        let note_chars: Vec<char> = note.chars().collect();
                        
                        string_chars.push(Span::styled(note_chars[0].to_string(), style));
                        if note_chars.len() > 1 {
                            // If the note has a sharp/flat, we draw it in the next column
                            if consumed_next {
                                // The next column was part of the number, so overwrite it
                                string_chars.push(Span::styled(note_chars[1].to_string(), style));
                                i += 1; // skip next column as we consumed it
                            } else {
                                // The next column was likely a dash. We just override it visually!
                                if i + 1 < chunk.len() {
                                    string_chars.push(Span::styled(note_chars[1].to_string(), style));
                                    i += 1;
                                }
                            }
                        } else if consumed_next {
                            // Fret was 2 digits (e.g. 10), but note is 1 char (e.g. D). We need to fill the second column with a dash
                            string_chars.push(Span::styled("-".to_string(), style));
                            i += 1;
                        }
                    } else {
                        string_chars.push(Span::styled(c.to_string(), style));
                    }
                } else {
                    string_chars.push(Span::styled(c.to_string(), style));
                }

                // Check cursor
                if global_col == app.editor.cursor.col && string_idx == app.editor.cursor.string {
                    cursor_visual_pos = Some((i + 2, start_y + string_idx)); // +2 for "E|"
                }

                i += 1;
            }
            lines.push(Line::from(string_chars));
        }

        // Add a blank line between blocks
        lines.push(Line::from(""));
        
    }

    (Text::from(lines), cursor_visual_pos)
}
