use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyEventState};
use std::fs;
use termtab::app::App;
use termtab::editor::Editor;

fn create_key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

fn create_char(c: char) -> KeyEvent {
    let mut modifiers = KeyModifiers::empty();
    if c.is_uppercase() {
        modifiers.insert(KeyModifiers::SHIFT);
    }
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

fn simulate_keys(app: &mut App, keys: &[KeyEvent]) {
    for key in keys {
        let _ = app.handle_key(*key);
    }
}

fn load_snapshot(path: &str) -> App {
    let json = fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
    let editor: Editor = serde_json::from_str(&json).unwrap_or_else(|_| Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']));
    App::new(editor, path.to_string())
}

#[test]
fn test_basic_navigation() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    simulate_keys(&mut app, &[
        create_char('l'),
        create_char('l'),
        create_char('j'),
    ]);
    assert_eq!(app.editor.cursor.col, 2);
    assert_eq!(app.editor.cursor.string, 1);
}

#[test]
fn test_continuous_replace() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    simulate_keys(&mut app, &[
        create_char('R'),
        create_char('3'),
        create_char('5'),
        create_char('7'),
        create_key(KeyCode::Esc),
    ]);
    assert_eq!(app.editor.document.get_char(0, 0), '3');
    assert_eq!(app.editor.document.get_char(1, 0), '5');
    assert_eq!(app.editor.document.get_char(2, 0), '7');
}

#[test]
fn test_insert_mode() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    let initial_cols = app.editor.document.total_global_cols();
    
    simulate_keys(&mut app, &[
        create_char('I'),
        create_char('3'),
        create_char('5'),
        create_char(' '),
        create_char('7'),
        create_key(KeyCode::Esc),
    ]);
    
    assert_eq!(app.editor.document.get_char(0, 0), '3');
    assert_eq!(app.editor.document.get_char(1, 0), '5');
    assert_eq!(app.editor.document.get_char(2, 0), '-'); // space inserts a dash column
    assert_eq!(app.editor.document.get_char(3, 0), '7');
    
    // Total cols should have increased by 4
    assert_eq!(app.editor.document.total_global_cols(), initial_cols + 4);
}

#[test]
fn test_enter_behavior() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    
    // Move to col 2, enter Insert
    simulate_keys(&mut app, &[
        create_char('l'),
        create_char('l'),
        create_char('I'),
        create_char('1'),
    ]);

    // Validate state right before Enter
    assert_eq!(app.editor.cursor.col, 3);
    assert_eq!(app.editor.cursor.string, 0);
    match &app.mode {
        termtab::app::Mode::Insert { start_col: 2 } => {}
        _ => panic!("Expected Insert mode with start_col 2"),
    }

    // Press Enter
    simulate_keys(&mut app, &[create_key(KeyCode::Enter)]);

    // Validate state immediately after Enter
    assert_eq!(app.editor.cursor.col, 2); // Returned to start_col
    assert_eq!(app.editor.cursor.string, 1); // Moved down a string
    match &app.mode {
        termtab::app::Mode::ContinuousReplace { start_col: 2 } => {}
        _ => panic!("Expected ContinuousReplace mode with start_col 2"),
    }

    // Type 2 and exit
    simulate_keys(&mut app, &[
        create_char('2'),
        create_key(KeyCode::Esc),
    ]);
    
    assert_eq!(app.editor.document.get_char(2, 0), '1');
    assert_eq!(app.editor.document.get_char(2, 1), '2');
    
    // Cursor should be right after '2', so col 3
    assert_eq!(app.editor.cursor.col, 3);
    assert_eq!(app.editor.cursor.string, 1);
}

#[test]
fn test_replace_mode_live_preview() {
    // Note: Live preview logic is mostly in tui.rs render code.
    // However, we can assert that Mode::Replace stores the buffer correctly.
    let mut app = load_snapshot("tests/fixtures/empty.json");
    
    simulate_keys(&mut app, &[
        create_char('r'),
        create_char('1'),
    ]);
    
    match &app.mode {
        termtab::app::Mode::Replace { buffer } => {
            assert_eq!(buffer, "1");
        }
        _ => panic!("Expected Replace mode with buffer '1'"),
    }
}

#[test]
fn test_undo_redo_continuous_mode() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    
    // Enter continuous mode and type 1, 2, 3
    simulate_keys(&mut app, &[
        create_char('R'),
        create_char('1'),
        create_char('2'),
        create_char('3'),
        create_key(KeyCode::Esc),
    ]);
    
    assert_eq!(app.editor.document.get_char(0, 0), '1');
    assert_eq!(app.editor.document.get_char(1, 0), '2');
    assert_eq!(app.editor.document.get_char(2, 0), '3');
    
    // Undo should remove ALL three keystrokes because they were grouped
    simulate_keys(&mut app, &[
        create_char('u'),
    ]);
    
    assert_eq!(app.editor.document.get_char(0, 0), '-');
    assert_eq!(app.editor.document.get_char(1, 0), '-');
    assert_eq!(app.editor.document.get_char(2, 0), '-');

    // Redo should put them back
    simulate_keys(&mut app, &[
        create_char('U'),
    ]);

    assert_eq!(app.editor.document.get_char(0, 0), '1');
    assert_eq!(app.editor.document.get_char(1, 0), '2');
    assert_eq!(app.editor.document.get_char(2, 0), '3');
}

#[test]
fn test_undo_redo_insert_mode() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    let initial_cols = app.editor.document.total_global_cols();
    
    // Enter insert mode, type 5, 5, 5, hit esc
    simulate_keys(&mut app, &[
        create_char('i'),
        create_char('5'),
        create_char('5'),
        create_char('5'),
        create_key(KeyCode::Esc),
    ]);
    
    assert_eq!(app.editor.document.total_global_cols(), initial_cols + 3);
    assert_eq!(app.editor.document.get_char(0, 0), '5');
    assert_eq!(app.editor.document.get_char(1, 0), '5');
    assert_eq!(app.editor.document.get_char(2, 0), '5');
    
    // Undo should remove all 3 insertions
    simulate_keys(&mut app, &[
        create_char('u'),
    ]);
    
    assert_eq!(app.editor.document.total_global_cols(), initial_cols);
    assert_eq!(app.editor.document.get_char(0, 0), '-');

    // Redo
    simulate_keys(&mut app, &[
        create_char('U'),
    ]);
    
    assert_eq!(app.editor.document.total_global_cols(), initial_cols + 3);
    assert_eq!(app.editor.document.get_char(0, 0), '5');
}

