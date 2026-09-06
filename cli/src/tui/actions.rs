use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::{App, theme};

pub fn render_confirm_delete(frame: &mut Frame, app: &App) {
    let Some(&idx) = app.filtered.get(app.selected) else {
        return;
    };
    let name = &app.rows[idx].name;

    let area = centered_rect(50, 15, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Delete session ")
        .style(theme::destructive());
    let text = format!("Delete \"{name}\"?\n\n[y] confirm   [n] cancel");
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center),
        area,
    );
}

pub fn render_help(frame: &mut Frame) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title(" Help ");
    let text = "\u{2191}/\u{2193}, j/k    navigate\n\
/               filter\n\
s               reveal/mask secrets\n\
e               edit in $EDITOR\n\
d               delete (confirm)\n\
q, Esc          quit\n\
?               toggle this help";
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [vertical] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [horizontal] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(vertical);
    horizontal
}
