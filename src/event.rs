/* Crossterm event handler — identical twin of solverforge-mail's event.rs.  */

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

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

        thread::spawn(move || {
            let mut next_tick = Instant::now() + tick_rate;
            loop {
                // Input must not postpone worker polling indefinitely.
                let now = Instant::now();
                if now >= next_tick {
                    if event_tx.send(Event::Tick).is_err() {
                        return;
                    }
                    next_tick = now + tick_rate;
                }
                if event::poll(next_tick.saturating_duration_since(Instant::now())).unwrap_or(false)
                {
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
                        Ok(CrosstermEvent::Resize(_, _))
                            if event_tx.send(Event::Resize).is_err() =>
                        {
                            return;
                        }
                        _ => {}
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    pub fn next(&self) -> anyhow::Result<Event> {
        self.rx.recv().map_err(Into::into)
    }

    /// A frame deadline wakes only the consumer; worker ticks keep their cadence.
    pub fn next_timeout(&self, timeout: Duration) -> anyhow::Result<Option<Event>> {
        match self.rx.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_deadline_does_not_generate_worker_ticks() {
        let (tx, rx) = mpsc::channel();
        let handler = EventHandler {
            rx,
            _tx: tx.clone(),
        };
        assert!(handler.next_timeout(Duration::ZERO).unwrap().is_none());
        tx.send(Event::Tick).unwrap();
        assert!(matches!(
            handler.next_timeout(Duration::ZERO).unwrap(),
            Some(Event::Tick)
        ));
        assert!(handler.next_timeout(Duration::ZERO).unwrap().is_none());
    }
}
