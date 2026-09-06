use owo_colors::OwoColorize;
use serde_json::Value;

/// Pretty-prints `value`, optionally with per-token coloring. A hand-rolled
/// walker rather than a syntax-highlighting crate — JSON is the only
/// format that needs this, so a small recursive printer is simpler and
/// lighter than pulling in a general-purpose highlighter.
pub fn render(value: &Value, colorize: bool) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0, colorize);
    out
}

fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_value(out: &mut String, value: &Value, depth: usize, colorize: bool) {
    match value {
        Value::Null => {
            if colorize {
                out.push_str(&"null".dimmed().to_string());
            } else {
                out.push_str("null");
            }
        }
        Value::Bool(b) => {
            let text = b.to_string();
            if colorize {
                out.push_str(&text.yellow().to_string());
            } else {
                out.push_str(&text);
            }
        }
        Value::Number(n) => {
            let text = n.to_string();
            if colorize {
                out.push_str(&text.cyan().to_string());
            } else {
                out.push_str(&text);
            }
        }
        Value::String(s) => {
            let quoted = format!("{s:?}");
            if colorize {
                out.push_str(&quoted.green().to_string());
            } else {
                out.push_str(&quoted);
            }
        }
        Value::Array(items) => write_array(out, items, depth, colorize),
        Value::Object(map) => write_object(out, map, depth, colorize),
    }
}

fn write_array(out: &mut String, items: &[Value], depth: usize, colorize: bool) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push_str("[\n");
    for (i, item) in items.iter().enumerate() {
        push_indent(out, depth + 1);
        write_value(out, item, depth + 1, colorize);
        if i + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    push_indent(out, depth);
    out.push(']');
}

fn write_object(
    out: &mut String,
    map: &serde_json::Map<String, Value>,
    depth: usize,
    colorize: bool,
) {
    if map.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push_str("{\n");
    let len = map.len();
    for (i, (key, val)) in map.iter().enumerate() {
        push_indent(out, depth + 1);
        let quoted_key = format!("{key:?}");
        if colorize {
            out.push_str(&quoted_key.blue().bold().to_string());
        } else {
            out.push_str(&quoted_key);
        }
        out.push_str(": ");
        write_value(out, val, depth + 1, colorize);
        if i + 1 < len {
            out.push(',');
        }
        out.push('\n');
    }
    push_indent(out, depth);
    out.push('}');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_plain_uncolored_output() {
        let value = json!({"a": 1, "b": [true, null, "x"]});
        let rendered = render(&value, false);
        assert!(rendered.contains("\"a\": 1"));
        assert!(rendered.contains("true"));
        assert!(rendered.contains("null"));
        assert!(rendered.contains("\"x\""));
    }

    #[test]
    fn empty_containers_render_compactly() {
        assert_eq!(render(&json!([]), false), "[]");
        assert_eq!(render(&json!({}), false), "{}");
    }

    #[test]
    fn colorized_output_contains_ansi_escapes() {
        let rendered = render(&json!({"a": 1}), true);
        assert!(rendered.contains("\u{1b}["));
    }
}
