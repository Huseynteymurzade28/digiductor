//! Loading animations: a braille spinner glyph plus a multi-line "digitizing"
//! boot sequence used to fill empty panes while the network is in flight.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Current spinner glyph for the given animation tick.
pub fn glyph(tick: u64) -> &'static str {
    FRAMES[(tick as usize) % FRAMES.len()]
}

/// A centred multi-line boot animation: rotating status text and a scanning
/// progress bar. Purely cosmetic — communicates "the link is alive".
pub fn boot_lines(tick: u64) -> Vec<Line<'static>> {
    let glyph = glyph(tick);

    const WIDTH: usize = 18;
    let filled = (tick as usize / 1) % (WIDTH + 1);
    let bar: String = (0..WIDTH)
        .map(|i| if i < filled { '▓' } else { '░' })
        .collect();

    const MSGS: [&str; 4] = [
        "ESTABLISHING UPLINK",
        "DIGITIZING DATA STREAM",
        "DECODING DIGI-CODE",
        "SYNCING DIGIVICE",
    ];
    let msg = MSGS[(tick as usize / 12) % MSGS.len()];

    let neon = Style::default().fg(theme::NEON);
    let cyan = Style::default()
        .fg(theme::CYAN)
        .add_modifier(Modifier::BOLD);

    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {glyph}  "), neon),
            Span::styled(msg, cyan),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [", theme::dim()),
            Span::styled(bar, neon),
            Span::styled("]", theme::dim()),
        ]),
        Line::from(Span::styled(
            format!("  {glyph} please stand by…"),
            theme::dim(),
        )),
    ]
}
