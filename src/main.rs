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
mod favorites;
mod network;
mod sprite;
mod theme;
mod ui;

use app::App;
use event::{Event, EventHandler};
use ratatui::DefaultTerminal;
use ratatui_image::picker::{Picker, ProtocolType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Must run *before* we take over stdin with the event reader.
    let picker = make_picker();

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

/// Pick how sprites are drawn: a real graphics protocol (kitty / iTerm2 /
/// sixel) or colored half-blocks, which work in any truecolor terminal and suit
/// the pixel-sprite look.
///
/// Probing the terminal is only done where a graphics protocol is likely.
/// `Picker::from_query_stdio` reads the terminal's reply on a helper thread; if
/// the terminal never answers (tmux, many plain terminals) that thread stays
/// blocked on stdin after the timeout and silently swallows key presses.
///
/// Override with `DIGIDUCTOR_GRAPHICS=on` (always probe) or `=off` (half-blocks).
fn make_picker() -> Picker {
    // `(8, 16)` is a reasonable default cell size for half-blocks.
    let halfblocks = || Picker::from_fontsize((8, 16));
    let probe = || Picker::from_query_stdio().unwrap_or_else(|_| halfblocks());

    match std::env::var("DIGIDUCTOR_GRAPHICS")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "1" | "on" | "true" => probe(),
        "0" | "off" | "false" | "halfblocks" => {
            // `from_fontsize` may still guess a protocol from env vars (e.g.
            // inside tmux), so pin it explicitly.
            let mut p = halfblocks();
            p.set_protocol_type(ProtocolType::Halfblocks);
            p
        }
        _ if likely_graphics_terminal() => probe(),
        _ => halfblocks(),
    }
}

/// Best-effort guess from the environment that the terminal speaks a graphics
/// protocol and will answer the capability query promptly.
fn likely_graphics_terminal() -> bool {
    use std::env::{var, var_os};
    if var_os("TMUX").is_some() || var_os("STY").is_some() {
        return false; // multiplexers often swallow the query
    }
    if var_os("KITTY_WINDOW_ID").is_some()
        || var_os("WEZTERM_EXECUTABLE").is_some()
        || var_os("GHOSTTY_RESOURCES_DIR").is_some()
        || var_os("ITERM_SESSION_ID").is_some()
    {
        return true;
    }
    let term = format!(
        "{} {}",
        var("TERM").unwrap_or_default(),
        var("TERM_PROGRAM").unwrap_or_default()
    )
    .to_lowercase();
    [
        "kitty", "ghostty", "wezterm", "iterm", "foot", "mlterm", "contour", "rio",
    ]
    .iter()
    .any(|t| term.contains(t))
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
