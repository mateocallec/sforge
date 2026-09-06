use owo_colors::OwoColorize;

/// Muted, not neon: 2xx green-ish, 3xx cyan-ish, 4xx yellow-ish, 5xx
/// red-ish — enough to scan a status at a glance without shouting.
pub fn render_status_line(status_code: u16, reason: &str, colorize: bool) -> String {
    let line = format!("HTTP/1.1 {status_code} {reason}");
    if !colorize {
        return line;
    }
    match status_code {
        200..=299 => line.green().to_string(),
        300..=399 => line.cyan().to_string(),
        400..=499 => line.yellow().to_string(),
        500..=599 => line.red().to_string(),
        _ => line,
    }
}

/// Headers in their original casing/order — symmetric with the byte-exact
/// philosophy on the request side.
pub fn render_headers(headers: &[(String, String)], colorize: bool) -> String {
    let mut out = String::new();
    for (name, value) in headers {
        if colorize {
            out.push_str(&name.bold().to_string());
        } else {
            out.push_str(name);
        }
        out.push_str(": ");
        out.push_str(value);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_format() {
        assert_eq!(render_status_line(200, "OK", false), "HTTP/1.1 200 OK");
    }

    #[test]
    fn headers_render_one_per_line() {
        let headers = vec![
            ("Content-Type".to_string(), "text/plain".to_string()),
            ("X-Debug".to_string(), "1".to_string()),
        ];
        let rendered = render_headers(&headers, false);
        assert_eq!(rendered, "Content-Type: text/plain\nX-Debug: 1\n");
    }
}
