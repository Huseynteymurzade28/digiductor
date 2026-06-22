//! Cyberpunk / "Digital World" palette and shared widget styling.
//!
//! The aesthetic: near-black background, neon green as the primary signal
//! colour (matrix rain), cyan for structure/labels, magenta for the "prior"
//! evolution branch, amber for warnings, and muted grey-green for chrome.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType};

pub const BG: Color = Color::Rgb(8, 10, 14);
pub const NEON: Color = Color::Rgb(57, 255, 100); // primary neon green
pub const CYAN: Color = Color::Rgb(0, 238, 222);
pub const MAGENTA: Color = Color::Rgb(255, 72, 184);
pub const AMBER: Color = Color::Rgb(245, 205, 70);
pub const DIM: Color = Color::Rgb(86, 122, 104); // muted grey-green
pub const FAINT: Color = Color::Rgb(48, 60, 58);
pub const FG: Color = Color::Rgb(200, 240, 215);

pub fn cyan() -> Style {
    Style::default().fg(CYAN)
}

pub fn dim() -> Style {
    Style::default().fg(DIM)
}

pub fn label() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

pub fn value() -> Style {
    Style::default().fg(FG)
}

/// A bordered panel in house style. `active` brightens the border so the user
/// can see which pane has focus.
pub fn panel(title: &str, active: bool) -> Block<'static> {
    let border = if active { NEON } else { FAINT };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(if active { NEON } else { CYAN })
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG))
}

/// Map the API's internal (Japanese-tradition) level names to the English terms
/// fans expect, which is also what our filter labels use.
pub fn english_level(api_level: &str) -> &str {
    match api_level {
        "Baby I" => "Fresh",
        "Baby II" => "In-Training",
        "Child" => "Rookie",
        "Adult" => "Champion",
        "Perfect" => "Ultimate",
        "Ultimate" => "Mega",
        other => other,
    }
}

/// Tint an attribute by faction colour for quick visual scanning.
pub fn attribute_color(attribute: &str) -> Color {
    match attribute {
        "Vaccine" => NEON,
        "Virus" => MAGENTA,
        "Data" => CYAN,
        "Free" => AMBER,
        _ => DIM,
    }
}
