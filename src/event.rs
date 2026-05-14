use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    FocusGained,
    FocusLost,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        tokio::spawn(async move {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key))
                            if tx.send(Event::Key(key)).is_err() => {
                                return;
                            }
                        Ok(CrosstermEvent::FocusGained)
                            if tx.send(Event::FocusGained).is_err() => {
                                return;
                            }
                        Ok(CrosstermEvent::FocusLost)
                            if tx.send(Event::FocusLost).is_err() => {
                                return;
                            }
                        _ => {}
                    }
                } else if tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        });

        Self { rx, _tx }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("event channel closed"))
    }
}
