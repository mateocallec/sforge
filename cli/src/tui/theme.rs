use ratatui::style::{Color, Modifier, Style};

const ACCENT: Color = Color::Cyan;
const MUTED_RED: Color = Color::Red;
const MUTED_GREEN: Color = Color::Green;
const DIM: Color = Color::DarkGray;

/// Restrained on purpose: one accent for the current row, muted red only
/// for a destructive confirmation, muted green only for a brief success
/// flash, dim gray for secondary text — nothing else.
pub fn selected_row() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn header() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn destructive() -> Style {
    Style::default().fg(MUTED_RED)
}

pub fn success() -> Style {
    Style::default().fg(MUTED_GREEN)
}

pub fn dim() -> Style {
    Style::default().fg(DIM)
}
