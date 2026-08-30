/* Crossterm event handler — identical twin of solverforge-mail's event.rs.  */

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};

/* Application-level events fed into the main loop. */
#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize,
    Tick,
}

/* Spawns a background thread that polls crossterm for terminal events */
/* and sends them through an `mpsc` channel. */
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    _tx: mpsc::Sender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();

        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                match event::read() {
                    Ok(CrosstermEvent::Key(key))
                        if key.kind == KeyEventKind::Press
                            && event_tx.send(Event::Key(key)).is_err() =>
                    {
                        return;
                    }
                    Ok(CrosstermEvent::Mouse(mouse))
                        if event_tx.send(Event::Mouse(mouse)).is_err() =>
                    {
                        return;
                    }
                    Ok(CrosstermEvent::Resize(_, _)) if event_tx.send(Event::Resize).is_err() => {
                        return;
                    }
                    _ => {}
                }
            } else if event_tx.send(Event::Tick).is_err() {
                return;
            }
        });

        Self { rx, _tx: tx }
    }

    pub fn next(&self) -> anyhow::Result<Event> {
        self.rx.recv().map_err(Into::into)
    }
}
