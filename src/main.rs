mod app;
mod document;
mod editor;
mod notes;
mod tui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io};
use app::App;
use editor::Editor;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: termtab <filename>");
        std::process::exit(1);
    }
    let filename = args[1].clone();

    let editor = if let Ok(json) = std::fs::read_to_string(&filename) {
        match serde_json::from_str(&json) {
            Ok(ed) => ed,
            Err(e) => {
                println!("Failed to parse {}: {}", filename, e);
                std::process::exit(1);
            }
        }
    } else {
        Editor::new_with_measures()
    };

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new(editor, filename);

    loop {
        terminal.draw(|f| tui::draw(f, &app))?;

        app.handle_events()?;

        if app.should_quit {
            break;
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
