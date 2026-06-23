//! Top-right pane: the full data sheet for the selected Digimon, with its
//! sprite rendered alongside (graphics protocol if the terminal supports one,
//! otherwise colored half-blocks).

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};

use crate::app::App;
use crate::network::api::Digimon;
use crate::theme;
use crate::ui::spinner;

/// Below this analyzer width we drop the sprite column and show text only.
const MIN_WIDTH_FOR_SPRITE: u16 = 46;
const SPRITE_COLS: u16 = 20;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let block = theme::panel("▣ DIGIMON ANALYZER", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Nothing selected yet → placeholder / boot animation.
    if app.detail.is_none() {
        let lines = if app.loading_detail {
            spinner::boot_lines(app.tick_count)
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  ◇ Select a Digimon from the index",
                    theme::dim(),
                )),
                Line::from(Span::styled(
                    "    to materialize its data here.",
                    theme::dim(),
                )),
            ]
        };
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }

    // Split off a sprite column when there's room. Sprites are ~square and a
    // terminal cell is roughly twice as tall as it is wide, so to fill the box
    // vertically (it stretches with the pane) the column needs about two columns
    // per row of height. We grow it with the pane but cap it at 45% of the width
    // so the data sheet keeps its room.
    let text_area = if inner.width >= MIN_WIDTH_FOR_SPRITE {
        let max_cols = (inner.width * 9 / 20).max(SPRITE_COLS);
        let square_cols = inner.height.saturating_sub(2).saturating_mul(2);
        let sprite_cols = square_cols.clamp(SPRITE_COLS, max_cols);
        let cols =
            Layout::horizontal([Constraint::Length(sprite_cols), Constraint::Min(0)]).split(inner);
        render_sprite(f, cols[0], app);
        cols[1]
    } else {
        inner
    };

    if let Some(d) = &app.detail {
        let para = Paragraph::new(data_sheet(d))
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll, 0));
        f.render_widget(para, text_area);
    }
}

/// Draw the boxed sprite (or a loading/placeholder glyph) in `area`.
fn render_sprite(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::dim())
        .title(Span::styled(" SPRITE ", theme::cyan()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(protocol) = app.image_state.as_mut() {
        // `Fit` preserves the sprite's aspect ratio within the cell box.
        let widget = StatefulImage::<StatefulProtocol>::default().resize(Resize::Fit(None));
        f.render_stateful_widget(widget, inner, protocol);
    } else {
        let text = if app.loading_image {
            format!("{} digitizing", spinner::glyph(app.tick_count))
        } else {
            "∅ no sprite".to_string()
        };
        let placeholder = Paragraph::new(Line::from(Span::styled(text, theme::dim())))
            .alignment(Alignment::Center);
        f.render_widget(placeholder, inner);
    }
}

fn data_sheet(d: &Digimon) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    // Name banner.
    lines.push(Line::from(Span::styled(
        format!("⟪ {} ⟫", d.name.to_uppercase()),
        Style::default()
            .fg(theme::NEON)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("#{:04}  ·  released {}", d.id, year(&d.release_date)),
        theme::dim(),
    )));
    lines.push(Line::from(""));

    // Stat rows.
    let level = d
        .primary_level()
        .map(|l| theme::english_level(l).to_string())
        .unwrap_or_else(|| "Unknown".into());
    lines.push(stat("LEVEL", &level, theme::NEON));

    if let Some(attr) = d.primary_attribute() {
        lines.push(stat("ATTRIBUTE", attr, theme::attribute_color(attr)));
    } else {
        lines.push(stat("ATTRIBUTE", "—", theme::DIM));
    }
    lines.push(stat("TYPE", &d.type_list(), theme::CYAN));
    lines.push(stat(
        "EVO PATHS",
        &format!(
            "{} prior · {} next",
            d.prior_evolutions.len(),
            d.next_evolutions.len()
        ),
        theme::MAGENTA,
    ));

    // Description.
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "── FIELD GUIDE ──────────────",
        theme::dim(),
    )));
    for para in d.english_description().split('\n') {
        lines.push(Line::from(Span::styled(para.to_string(), theme::value())));
    }

    lines
}

fn stat(key: &str, value: &str, color: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<11}"), theme::label()),
        Span::styled("▎ ", theme::dim()),
        Span::styled(
            value.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn year(raw: &str) -> String {
    if raw.is_empty() {
        "unknown".into()
    } else {
        raw.chars().take(4).collect()
    }
}
