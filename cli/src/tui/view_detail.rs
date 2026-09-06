use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{App, theme};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" Detail ");

    let Some(session) = &app.detail else {
        frame.render_widget(
            Paragraph::new("Select a session.")
                .block(block)
                .style(theme::dim()),
            area,
        );
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled("Auth", theme::header())));
    match session.auth.kind {
        Some(kind) => lines.push(Line::from(format!("  {kind:?}"))),
        None => lines.push(Line::from(Span::styled("  none", theme::dim()))),
    }
    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled(
        format!("Headers ({})", session.headers.len()),
        theme::header(),
    )));
    for h in &session.headers {
        let value = if app.reveal_secrets {
            h.value.clone()
        } else {
            mask(&h.value)
        };
        lines.push(Line::from(format!("  {}: {}", h.name, value)));
    }
    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled(
        format!("Cookies ({})", session.cookies.len()),
        theme::header(),
    )));
    for c in &session.cookies {
        let value = if app.reveal_secrets {
            c.value.clone()
        } else {
            mask(&c.value)
        };
        let mut flags = Vec::new();
        if c.secure {
            flags.push("Secure");
        }
        if c.http_only {
            flags.push("HttpOnly");
        }
        let flags_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };
        lines.push(Line::from(format!(
            "  {}={}  domain={} path={}{}",
            c.name, value, c.domain, c.path, flags_str
        )));
    }

    lines.push(Line::raw(""));
    let footer = if app.reveal_secrets {
        "press 's' to mask secrets"
    } else {
        "press 's' to reveal secrets"
    };
    lines.push(Line::from(Span::styled(footer, theme::dim())));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Cookie/header values are live pentest credentials on screen — masked by
/// default, byte-safe (operates on chars, not byte slices, since values
/// aren't guaranteed ASCII).
fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 6 {
        return "*".repeat(chars.len());
    }
    let prefix: String = chars[..2].iter().collect();
    let suffix: String = chars[chars.len() - 2..].iter().collect();
    format!("{prefix}****{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_short_values_completely() {
        assert_eq!(mask("abc"), "***");
    }

    #[test]
    fn masks_long_values_keeping_prefix_and_suffix() {
        assert_eq!(mask("abcdefgh"), "ab****gh");
    }

    #[test]
    fn does_not_panic_on_multibyte_characters() {
        let masked = mask("caf\u{00e9}caf\u{00e9}caf\u{00e9}");
        assert!(masked.contains("****"));
    }
}
