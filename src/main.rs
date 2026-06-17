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
    
    let mut print_mode = false;
    let mut filename = String::new();

    for arg in &args[1..] {
        if arg == "--print" {
            print_mode = true;
        } else {
            filename = arg.clone();
        }
    }

    if filename.is_empty() {
        println!("Usage: termtab [--print] <filename>");
        std::process::exit(1);
    }

    let editor = if let Ok(json) = std::fs::read_to_string(&filename) {
        match serde_json::from_str(&json) {
            Ok(ed) => ed,
            Err(e) => {
                println!("Failed to parse {}: {}", filename, e);
                std::process::exit(1);
            }
        }
    } else {
        if print_mode {
            println!("File not found: {}", filename);
            std::process::exit(1);
        }
        Editor::new()
    };

    if print_mode {
        let (width, _) = crossterm::terminal::size().unwrap_or((80, 24));
        let wrap_width = if width > 4 { (width - 4) as usize } else { 80 };
        print!("{}", editor.document.dump_to_string(wrap_width));
        return Ok(());
    }

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
