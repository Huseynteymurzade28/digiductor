//! Digimon "Digital World" palette and shared widget styling.
//!
//! Built from the official Digimon colour palette: Neon Green is the primary
//! signal colour, Digital Blue carries structure/labels, Pixel Pink marks the
//! "prior" evolution branch, Cyber Purple frames the active node, Data Yellow
//! warns, and Virtual Red flags errors. Background is a near-black blue.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType};

pub const BG: Color = Color::Rgb(7, 9, 18); // near-black digital blue
pub const NEON: Color = Color::Rgb(127, 255, 0); // Neon Green — primary signal
pub const CYAN: Color = Color::Rgb(0, 132, 255); // Digital Blue — structure/labels
pub const MAGENTA: Color = Color::Rgb(255, 20, 147); // Pixel Pink — prior branch
pub const PURPLE: Color = Color::Rgb(155, 48, 255); // Cyber Purple — active node
pub const AMBER: Color = Color::Rgb(255, 215, 0); // Data Yellow — warnings
pub const RED: Color = Color::Rgb(255, 36, 0); // Virtual Red — errors
pub const DIM: Color = Color::Rgb(96, 120, 168); // muted digital blue-grey
pub const FAINT: Color = Color::Rgb(40, 52, 84);
pub const FG: Color = Color::Rgb(214, 232, 255);

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
        "Virus" => PURPLE,
        "Data" => CYAN,
        "Free" => AMBER,
        _ => DIM,
    }
}
