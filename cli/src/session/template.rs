/// Renders the commented TOML template used when `session setup "<name>"`
/// is run with no inline data flags and no session by that name exists yet
/// — the file opened fresh in `$EDITOR`.
pub fn render(id: &str, name: &str, created_at: &str) -> String {
    let name = escape_toml_string(name);
    format!(
        r#"# SessionForge session file — safe to edit by hand.
# Identified internally by UUID; editing "name" here does not rename the
# session as sforge sees it (re-run `sforge session setup "<name>"` instead).

[meta]
id = "{id}"
name = "{name}"
created_at = "{created_at}"
updated_at = "{created_at}"

[auth]
# type = "basic" | "bearer" | "digest"
# username = "admin"
# password = "s3cr3t"

# [[headers]]
# name = "X-Debug"
# value = "1"

# [[cookies]]
# name = "session_id"
# value = "abc123"
# domain = "app.target.com"
# path = "/"
# secure = true
# http_only = true
# same_site = "Lax"
# expires = "2026-12-31T23:59:59Z"
"#
    )
}

fn escape_toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_quotes_and_backslashes_in_the_name() {
        let rendered = render("id-1", "weird \"name\" \\ here", "2026-01-01T00:00:00Z");
        assert!(rendered.contains(r#"name = "weird \"name\" \\ here""#));
    }
}
