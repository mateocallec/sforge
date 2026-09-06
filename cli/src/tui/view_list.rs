use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use super::{App, theme};

/// Fixed-width suffix appended to every row: " MM-DD HH:MM  c:NN h:NN".
const SUFFIX_WIDTH: usize = 24;
const MIN_NAME_WIDTH: usize = 8;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.rows.is_empty() {
        " Sessions ".to_string()
    } else {
        format!(" Sessions ({}) ", app.rows.len())
    };
    let mut block = Block::default().borders(Borders::ALL).title(title);
    if !app.filter.is_empty() {
        block = block.title_bottom(format!(" /{} ", app.filter));
    }

    if app.filtered.is_empty() {
        let text = if app.rows.is_empty() {
            "No sessions yet.\n\nCreate one with:\nsforge session setup \"<name>\" ..."
        } else {
            "No sessions match the filter."
        };
        frame.render_widget(Paragraph::new(text).block(block).style(theme::dim()), area);
        return;
    }

    // Inner width available once borders are subtracted; the name column
    // shrinks to fit whatever's left after the fixed-width suffix, rather
    // than a hardcoded width that clips on a narrow terminal.
    let inner_width = area.width.saturating_sub(2) as usize;
    let name_width = inner_width.saturating_sub(SUFFIX_WIDTH).max(MIN_NAME_WIDTH);

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(pos, &idx)| {
            let row = &app.rows[idx];
            let line = format!(
                "{:<name_width$} {}  c:{:<2} h:{:<2}",
                truncate(&row.name, name_width),
                short_timestamp(&row.updated_at),
                row.cookie_count,
                row.header_count,
            );
            let style = if pos == app.selected {
                theme::selected_row()
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    frame.render_widget(List::new(items).block(block), area);
}

/// `updated_at` arrives as "YYYY-MM-DD HH:MM"; the list only has room for
/// "MM-DD HH:MM" (the full date is one keypress away in the detail pane).
fn short_timestamp(full: &str) -> &str {
    full.get(5..).unwrap_or(full)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{head}\u{2026}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_timestamp_drops_the_year() {
        assert_eq!(short_timestamp("2026-09-04 18:04"), "09-04 18:04");
    }

    #[test]
    fn truncate_leaves_short_strings_untouched() {
        assert_eq!(truncate("abc", 10), "abc");
    }

    #[test]
    fn truncate_adds_an_ellipsis_for_long_strings() {
        assert_eq!(truncate("abcdefgh", 5), "abcd\u{2026}");
    }
}
