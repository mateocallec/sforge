/// curl's own `-v` convention (`>` sent, `<` received) — recognizable at a
/// glance to anyone who already reads curl's verbose output.
pub fn render_sent(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut lines: Vec<&str> = text.split("\r\n").collect();
    // A bodyless request's bytes end "...\r\n\r\n", which splits into two
    // trailing empty entries; collapse them into the single blank "> "
    // line that actually represents the header/body separator.
    if lines.len() >= 2 && lines[lines.len() - 1].is_empty() && lines[lines.len() - 2].is_empty() {
        lines.pop();
    }

    let mut out = String::new();
    for line in lines {
        out.push_str("> ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub fn render_received_headers(status_line: &str, headers: &[(String, String)]) -> String {
    let mut out = String::new();
    out.push_str("< ");
    out.push_str(status_line);
    out.push('\n');
    for (name, value) in headers {
        out.push_str("< ");
        out.push_str(name);
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
    fn sent_lines_are_prefixed_with_gt() {
        let rendered = render_sent(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(rendered.contains("> GET / HTTP/1.1\n"));
        assert!(rendered.contains("> Host: x\n"));
    }

    #[test]
    fn bodyless_request_shows_exactly_one_trailing_blank_line() {
        let rendered = render_sent(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n");
        assert_eq!(rendered, "> GET / HTTP/1.1\n> Host: x\n> \n");
    }

    #[test]
    fn received_lines_are_prefixed_with_lt() {
        let headers = vec![("Content-Type".to_string(), "text/plain".to_string())];
        let rendered = render_received_headers("HTTP/1.1 200 OK", &headers);
        assert_eq!(rendered, "< HTTP/1.1 200 OK\n< Content-Type: text/plain\n");
    }
}
