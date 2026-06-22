//! Event handling module.
//!
//! Multiplexes three asynchronous sources into a single channel that the main
//! loop awaits:
//!   * terminal input (crossterm `EventStream`),
//!   * a steady animation `Tick` (drives the spinner / boot animation),
//!   * `Net` results pushed in by spawned `reqwest` tasks.
//!
//! Because everything funnels through one `mpsc` receiver, the render loop never
//! blocks on the network: a slow request simply means `Tick`s keep flowing and
//! the UI keeps animating.

use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;

use crate::network::api::NetMessage;

/// Animation cadence. 100ms ≈ 10fps, smooth enough for a braille spinner.
const TICK_RATE: Duration = Duration::from_millis(100);

/// A unit of work for the main loop.
pub enum Event {
    /// Animation heartbeat.
    Tick,
    /// A key was pressed.
    Key(KeyEvent),
    /// The terminal was resized (triggers a redraw).
    Resize,
    /// An async network task finished.
    Net(NetMessage),
}

/// Owns the event channel. Hand out clones of `sender()` to network tasks so
/// their results re-enter the same loop as `Event::Net`.
pub struct EventHandler {
    sender: mpsc::UnboundedSender<Event>,
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let task_tx = sender.clone();

        // Background task: pump input + ticks. Network tasks feed the same
        // channel directly via `sender()`.
        tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick = tokio::time::interval(TICK_RATE);
            loop {
                let tick_delay = tick.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = tick_delay => {
                        if task_tx.send(Event::Tick).is_err() {
                            break; // receiver dropped — app is shutting down
                        }
                    }
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(CrosstermEvent::Key(key))) => {
                                // Ignore key-release / repeat noise on platforms
                                // that report them.
                                if key.kind == KeyEventKind::Press
                                    && task_tx.send(Event::Key(key)).is_err()
                                {
                                    break;
                                }
                            }
                            Some(Ok(CrosstermEvent::Resize(_, _))) => {
                                let _ = task_tx.send(Event::Resize);
                            }
                            Some(Err(_)) | None => {}
                            _ => {}
                        }
                    }
                }
            }
        });

        Self { sender, receiver }
    }

    /// A cloneable handle for spawned network tasks to report back through.
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }

    /// Await the next event. Falls back to `Tick` if the channel ever closes,
    /// keeping the render loop alive for a graceful exit.
    pub async fn next(&mut self) -> Event {
        self.receiver.recv().await.unwrap_or(Event::Tick)
    }
}
