//! UI layer: top-level layout plus the banner and status bar.
//!
//! Frame layout:
//! ```text
//! ┌──────────── banner ────────────┐
//! │ index │      analyzer          │
//! │ (left)├────────────────────────┤
//! │       │   evolution matrix     │
//! └──────────── status ────────────┘
//! ```

mod detail;
mod evolution;
mod list;
mod spinner;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::theme;

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Paint the whole frame with the dark cyber background first.
    f.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

    let chunks = Layout::vertical([
        Constraint::Length(3), // banner
        Constraint::Min(0),    // body
        Constraint::Length(1), // status bar
    ])
    .split(area);

    render_banner(f, chunks[0], app);

    let body = Layout::horizontal([Constraint::Length(38), Constraint::Min(0)]).split(chunks[1]);
    list::render(f, body[0], app);

    let right = Layout::vertical([Constraint::Percentage(54), Constraint::Percentage(46)])
        .split(body[1]);
    detail::render(f, right[0], app);
    evolution::render(f, right[1], app);

    render_status(f, chunks[2], app);
}

fn render_banner(f: &mut Frame, area: Rect, app: &App) {
    let spin = if app.spinner_active() {
        format!(" {} LIVE", spinner::glyph(app.tick_count))
    } else {
        " ◉ READY".to_string()
    };

    let title = Line::from(vec![
        Span::styled("▟█ ", Style::default().fg(theme::NEON)),
        Span::styled(
            "DIGIDUCTOR",
            Style::default()
                .fg(theme::NEON)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ░▒▓ ", theme::dim()),
        Span::styled(
            "DIGITAL MONSTER ENCYCLOPEDIA",
            Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ▓▒░", theme::dim()),
    ]);

    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::Double)
        .border_style(Style::default().fg(theme::FAINT))
        .title(Span::styled(
            spin,
            Style::default()
                .fg(theme::NEON)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Right)
        .style(Style::default().bg(theme::BG));

    f.render_widget(
        Paragraph::new(title)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([Constraint::Min(0), Constraint::Length(area.width / 2)])
        .split(area);

    // Left: status / error feedback.
    let left = if let Some(err) = &app.error {
        Line::from(vec![
            Span::styled(" ✖ ", Style::default().fg(theme::MAGENTA)),
            Span::styled(
                err.clone(),
                Style::default()
                    .fg(theme::AMBER)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ▸ ", Style::default().fg(theme::NEON)),
            Span::styled(app.status.clone(), theme::cyan()),
            Span::styled(format!("   filters: {}", app.filter_summary()), theme::dim()),
        ])
    };
    f.render_widget(Paragraph::new(left), cols[0]);

    // Right: contextual key hints.
    let hints = match app.mode {
        InputMode::Search => "type · ⏎ apply · esc cancel",
        InputMode::Browse => "↑↓ move · / search · l lvl · a attr · x clear · r reload · q quit",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(format!("{hints} "), theme::dim())))
            .alignment(ratatui::layout::Alignment::Right),
        cols[1],
    );
}
