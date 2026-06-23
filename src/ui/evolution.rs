//! Bottom-right pane: the branched evolution graph.
//!
//! Digimon evolution is *not* a straight line — one Digimon may digivolve from
//! several predecessors and into several successors. We render that as a
//! top-to-bottom tree with the current Digimon boxed in the middle:
//!
//! ```text
//!   PRIOR  ├─ Koromon
//!          └─ Tsunomon
//!              │
//!          ╔═══▼════╗
//!          ║ AGUMON ║
//!          ╚═══╤════╝
//!              │
//!    NEXT  ├─ Greymon
//!          ├─ GeoGreymon
//!          └─ Tyrannomon  (with the Digi-Egg of Courage)
//! ```

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::state::MAX_EVOLUTIONS_SHOWN;
use crate::app::App;
use crate::network::api::{Digimon, Evolution};
use crate::theme;

/// Cap branches so a Digimon with dozens of evolutions doesn't overflow.
const MAX_BRANCHES: usize = MAX_EVOLUTIONS_SHOWN;
/// Column at which the trunk / connector `│` sits.
const STEM: usize = 9;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.evo_focus;
    let block = theme::panel("⇄ EVOLUTION MATRIX", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(d) = &app.detail else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  awaiting digivolution data…",
                theme::dim(),
            ))),
            inner,
        );
        return;
    };

    f.render_widget(Paragraph::new(graph(d, app.evo_selected_id())), inner);
}

fn graph(d: &Digimon, selected: Option<u32>) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    branch_block("PRIOR", &d.prior_evolutions, theme::MAGENTA, selected, &mut lines);
    lines.push(connector(!d.prior_evolutions.is_empty()));
    current_box(d, &mut lines);
    lines.push(connector(!d.next_evolutions.is_empty()));
    branch_block("NEXT", &d.next_evolutions, theme::NEON, selected, &mut lines);

    lines
}

/// Render one side of the tree (the prior- or next-evolution fan-out).
fn branch_block(
    label: &str,
    items: &[Evolution],
    color: Color,
    selected: Option<u32>,
    out: &mut Vec<Line<'static>>,
) {
    if items.is_empty() {
        out.push(Line::from(vec![
            Span::styled(format!("{label:>8} "), theme::dim()),
            Span::styled("┄ none recorded", theme::dim()),
        ]));
        return;
    }

    let shown = items.len().min(MAX_BRANCHES);
    let mid = shown / 2;

    for (i, ev) in items.iter().take(shown).enumerate() {
        let branch = if shown == 1 {
            "──"
        } else if i == 0 {
            "┌─"
        } else if i == shown - 1 {
            "└─"
        } else {
            "├─"
        };
        let lead = if i == mid {
            Span::styled(
                format!("{label:>8} "),
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(" ".repeat(STEM))
        };

        // Highlight the entry under the navigation cursor.
        let highlighted = ev.id.is_some() && ev.id == selected;
        let name_style = if highlighted {
            Style::default()
                .fg(theme::BG)
                .bg(color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        };

        let mut spans = vec![
            lead,
            Span::styled(branch, theme::dim()),
            Span::styled(
                format!(" {}{} ", if highlighted { "▶ " } else { "" }, ev.digimon),
                name_style,
            ),
        ];
        if !ev.condition.trim().is_empty() {
            spans.push(Span::styled(format!("  ({})", condense(&ev.condition)), theme::dim()));
        }
        out.push(Line::from(spans));
    }

    if items.len() > shown {
        out.push(Line::from(vec![
            Span::raw(" ".repeat(STEM)),
            Span::styled(format!("… +{} more", items.len() - shown), theme::dim()),
        ]));
    }
}

/// The trunk between a branch block and the current-node box.
fn connector(present: bool) -> Line<'static> {
    let ch = if present { "│" } else { "·" };
    Line::from(vec![
        Span::raw(" ".repeat(STEM)),
        Span::styled(ch, theme::dim()),
    ])
}

/// The boxed, highlighted current Digimon at the centre of the graph.
fn current_box(d: &Digimon, out: &mut Vec<Line<'static>>) {
    let level = d
        .primary_level()
        .map(|l| theme::english_level(l).to_string())
        .unwrap_or_else(|| "?".into());
    let attr = d.primary_attribute().unwrap_or("—");
    let subtitle = format!("{level} · {attr}");

    let inner_w = d.name.chars().count().max(subtitle.chars().count()) + 4;
    // Left-pad so the box's vertical centre lines up under the trunk `│`.
    let pad = " ".repeat(STEM.saturating_sub(inner_w / 2 + 1));

    let frame = Style::default()
        .fg(theme::PURPLE)
        .add_modifier(Modifier::BOLD);

    out.push(Line::from(vec![
        Span::raw(pad.clone()),
        Span::styled(format!("╔{}╗", "═".repeat(inner_w)), frame),
    ]));
    out.push(Line::from(vec![
        Span::raw(pad.clone()),
        Span::styled("║", frame),
        Span::styled(
            format!("{:^inner_w$}", d.name),
            Style::default()
                .fg(theme::NEON)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("║", frame),
    ]));
    out.push(Line::from(vec![
        Span::raw(pad.clone()),
        Span::styled("║", frame),
        Span::styled(format!("{subtitle:^inner_w$}"), theme::dim()),
        Span::styled("║", frame),
    ]));
    out.push(Line::from(vec![
        Span::raw(pad),
        Span::styled(format!("╚{}╝", "═".repeat(inner_w)), frame),
    ]));
}

/// Trim an evolution condition to keep one line.
fn condense(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() > 38 {
        let mut t: String = s.chars().take(35).collect();
        t.push('…');
        t
    } else {
        s.to_string()
    }
}
