use std::io::{self, IsTerminal, Read};
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod query;
mod ui;

#[derive(Parser)]
struct Args {
    /// Optional JSON file to query
    file: Option<String>,
    /// Minimum number of lines for the query input area
    #[arg(short = 'n', long, default_value_t = 1)]
    min_lines: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let json_string = if let Some(path) = &args.file {
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {e}", path))?
    } else if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        eprintln!("Usage: live_jq [FILE] or pipe JSON via stdin");
        std::process::exit(1);
    };

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(json_string, args.min_lines);

    match query::run_query(&app.input_json, "") {
        Ok(out) => {
            app.output = out;
            app.error = None;
        }
        Err(e) => {
            app.error = Some(e);
        }
    }

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if app.focus == app::Focus::QueryInput {
                    match key.code {
                        KeyCode::Char(c) => app.input_char(c),
                        KeyCode::Backspace => app.input_backspace(),
                        KeyCode::Enter => app.input_enter(),
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                            app.handle_scroll(key.code);
                        }
                        _ => {}
                    }
                }

                match key.code {
                    KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => app.toggle_focus(),
                    _ => {}
                }

                if app.focus == app::Focus::QueryInput {
                    match query::run_query(&app.input_json, &app.query_buf) {
                        Ok(out) => {
                            app.output = out;
                            app.error = None;
                        }
                        Err(e) => {
                            app.error = Some(e);
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
