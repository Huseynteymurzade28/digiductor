//! Left pane: the searchable, filterable Digimon index.
//!
//! Layout inside the panel:
//!   ┌ search box ┐
//!   ┌ filter bar ┐
//!   └ scrollable list / boot animation ┘

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::theme;
use crate::ui::spinner;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let searching = app.mode == InputMode::Search;
    let block = theme::panel("◈ DIGI-INDEX", searching);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1), // search
        Constraint::Length(1), // filters
        Constraint::Length(1), // count / divider
        Constraint::Min(0),    // list
    ])
    .split(inner);

    render_search(f, rows[0], app, searching);
    render_filters(f, rows[1], app);
    render_count(f, rows[2], app);
    render_list(f, rows[3], app);
}

fn render_search(f: &mut Frame, area: Rect, app: &App, searching: bool) {
    let cursor = if searching && app.tick_count % 10 < 5 {
        "█"
    } else {
        ""
    };
    let query = if searching { &app.input } else { &app.search };
    let shown = if query.is_empty() && !searching {
        Span::styled("type / to search…", theme::dim())
    } else {
        Span::styled(format!("{query}{cursor}"), theme::value())
    };
    let line = Line::from(vec![
        Span::styled("⌕ ", Style::default().fg(theme::NEON)),
        shown,
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_filters(f: &mut Frame, area: Rect, app: &App) {
    let chip = |k: &str, v: &str| {
        let active = v != "ALL";
        let style = if active {
            Style::default()
                .fg(theme::BG)
                .bg(theme::NEON)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::dim()
        };
        vec![
            Span::styled(format!("{k}:"), theme::dim()),
            Span::styled(format!("[{v}]"), style),
            Span::raw(" "),
        ]
    };
    let mut spans = chip("LVL", app.level_filter.label());
    spans.extend(chip("ATR", app.attribute_filter.label()));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_count(f: &mut Frame, area: Rect, app: &App) {
    let text = if app.loading_list && app.list.is_empty() {
        format!("{} querying…", spinner::glyph(app.tick_count))
    } else {
        format!("◤ {} / {} records ◢", app.list.len(), app.total_elements)
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(text, theme::cyan()))),
        area,
    );
}

fn render_list(f: &mut Frame, area: Rect, app: &mut App) {
    if app.list.is_empty() {
        let lines = if app.loading_list {
            spinner::boot_lines(app.tick_count)
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled("  ∅ no matches", theme::dim())),
                Line::from(Span::styled("  press 'x' to clear filters", theme::dim())),
            ]
        };
        f.render_widget(Paragraph::new(lines), area);
        return;
    }

    let selected = app.list_state.selected();
    let items: Vec<ListItem> = app
        .list
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let is_sel = Some(i) == selected;
            let marker = if is_sel { "▶ " } else { "  " };
            let name_style = if is_sel {
                Style::default()
                    .fg(theme::NEON)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::value()
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme::NEON)),
                Span::styled(format!("#{:<4} ", d.id), theme::dim()),
                Span::styled(d.name.clone(), name_style),
            ]))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme::FAINT)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, area, &mut app.list_state);
}
