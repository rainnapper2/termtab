use termtab::app::App;
use termtab::editor::Editor;
use std::fs;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyEventState};

fn load_snapshot(path: &str) -> App {
    let json = fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
    let editor: Editor = serde_json::from_str(&json).unwrap_or_else(|_| Editor::new(vec!['e', 'B', 'G', 'D', 'A', 'E']));
    App::new(editor, path.to_string())
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

#[test]
fn test_undo_redo_insert_delete_column() {
    let mut app = load_snapshot("tests/fixtures/empty.json");
    let initial_cols = app.editor.document.total_global_cols();
    
    // Insert column >
    simulate_keys(&mut app, &[create_char('>')]);
    assert_eq!(app.editor.document.total_global_cols(), initial_cols + 1);
    
    // Undo
    simulate_keys(&mut app, &[create_char('u')]);
    assert_eq!(app.editor.document.total_global_cols(), initial_cols);

    // Redo
    simulate_keys(&mut app, &[create_char('U')]);
    assert_eq!(app.editor.document.total_global_cols(), initial_cols + 1);

    // Delete column <
    simulate_keys(&mut app, &[create_char('<')]);
    assert_eq!(app.editor.document.total_global_cols(), initial_cols);
    
    // Undo
    simulate_keys(&mut app, &[create_char('u')]);
    assert_eq!(app.editor.document.total_global_cols(), initial_cols + 1);
    
    // Redo
    simulate_keys(&mut app, &[create_char('U')]);
    assert_eq!(app.editor.document.total_global_cols(), initial_cols);
}
