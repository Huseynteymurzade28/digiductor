//! digiductor — a cyberpunk terminal Digimon encyclopedia.
//!
//! Architecture (clean, layered):
//!   * `network` — typed Digi-API models and async `reqwest` fetchers.
//!   * `cache`   — two-tier (memory + JSON-on-disk) record cache.
//!   * `app`     — state machine + reducer (input → state, net result → state).
//!   * `event`   — multiplexes input, animation ticks, and network results.
//!   * `ui`      — Ratatui render layer (index / analyzer / evolution matrix).
//!   * `theme`   — the neon-on-black palette and shared widget styling.
//!
//! The render loop never blocks: network work is spawned onto Tokio and its
//! results arrive as just another event, so the UI keeps animating throughout.

mod app;
mod cache;
mod event;
mod network;
mod theme;
mod ui;

use app::App;
use event::{Event, EventHandler};
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Detect the terminal's graphics protocol (kitty / iTerm2 / sixel) and font
    // size *before* we take over stdin with the event reader — the query reads
    // an escape-sequence response from the terminal. Terminals without graphics
    // support fall back to colored half-blocks, which work anywhere truecolor
    // does and suit the pixel-sprite aesthetic.
    // `(8, 16)` is a reasonable default cell size; the protocol then defaults to
    // half-blocks when no graphics protocol is detected.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::from_fontsize((8, 16)));

    // `ratatui::init` installs a panic hook that restores the terminal, so a
    // crash won't leave the user's shell in raw mode.
    let mut terminal = ratatui::init();
    let mut events = EventHandler::new();
    let mut app = App::new(events.sender(), picker);
    app.bootstrap();

    let result = run(&mut terminal, &mut app, &mut events).await;

    ratatui::restore();
    result
}

async fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    events: &mut EventHandler,
) -> anyhow::Result<()> {
    while app.running {
        terminal.draw(|f| ui::render(f, app))?;

        match events.next().await {
            Event::Tick => app.on_tick(),
            Event::Key(key) => app.on_key(key),
            Event::Resize => {} // next draw picks up the new size
            Event::Net(msg) => app.on_net(msg),
        }
    }
    Ok(())
}
