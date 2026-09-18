/* Planner123 — spiffy TUI calendar with local SQLite + Google Calendar.  Entry point: sets up the terminal, builds the tokio runtime, starts the event loop.  */

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use planner123::app::App;
use planner123::event::{Event, EventHandler};
use planner123::{notifications, ui};

fn main() -> Result<()> {
    // Install panic hook that restores the terminal before printing the panic.
    install_panic_hook();

    // Build the tokio runtime before anything else (needed by App::new and background tasks).
    let rt = tokio::runtime::Runtime::new()?;
    let rt_handle = rt.handle().clone();

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the application inside the tokio runtime context
    let result = rt.block_on(async { run(&mut terminal, rt_handle).await });

    // Restore terminal regardless of exit reason
    restore_terminal()?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rt: tokio::runtime::Handle,
) -> Result<()> {
    let mut app = App::new(rt);

    // Spawn the notification reminder background task
    let events_arc = app.events_arc.clone();
    notifications::spawn_reminder_task(events_arc);

    let events = EventHandler::new(Duration::from_millis(250));

    while app.running {
        // Draw frame
        terminal.draw(|frame| ui::render(&mut app, frame))?;

        // Handle event
        match events.next()? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(_mouse) => {
                // Mouse support: future enhancement
            }
            Event::Resize => {
                // Terminal resize: ratatui handles automatically on next draw
            }
            Event::Tick => {
                app.handle_tick();
            }
        }
    }

    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}
