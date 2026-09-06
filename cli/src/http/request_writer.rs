use std::collections::HashSet;

/// One header the caller explicitly wants on the wire, in the order given.
/// `Suppress` is curl's `-H "Name:"` convention — omit the header entirely,
/// even overriding a default that would otherwise add it. `Set` with an
/// empty value is `-H "Name;"` — send the header with nothing after the
/// colon.
#[derive(Debug, Clone)]
pub enum HeaderDirective {
    Set(String, String),
    Suppress(String),
}

impl HeaderDirective {
    fn name(&self) -> &str {
        match self {
            HeaderDirective::Set(name, _) => name,
            HeaderDirective::Suppress(name) => name,
        }
    }
}

/// Everything needed to compute the exact header set for one request: the
/// caller's own directives (session headers, then `-H` flags, in that
/// order — never reordered, never deduplicated), and the fallback defaults
/// used to fill in whatever the caller's own headers don't already cover.
pub struct RequestSpec {
    pub method: String,
    pub target: String,
    pub directives: Vec<HeaderDirective>,
    pub host_default: String,
    pub user_agent_default: String,
    pub accept_default: String,
    /// `Some(..)` only when `--compressed` was passed; when `None`, no
    /// Accept-Encoding default is added at all (a manual `-H
    /// "Accept-Encoding: ..."` is unaffected either way).
    pub accept_encoding_default: Option<String>,
    pub connection_default: String,
    /// Pre-joined `Cookie:` value (session cookies, then request-time
    /// cookies, in that order — duplicates intentionally not deduped).
    pub cookie_default: Option<String>,
    /// Content-Type implied by how the body was constructed (e.g.
    /// `application/x-www-form-urlencoded` for `-d`, a multipart boundary
    /// for `-F`), used only when the caller hasn't set their own.
    pub content_type_default: Option<String>,
    pub body: Vec<u8>,
    /// Forces `Transfer-Encoding: chunked` framing even though the body's
    /// length is fully known upfront — a deliberate parser-differential
    /// test `--chunked` exposes that plain curl does not.
    pub force_chunked: bool,
}

/// A fully-resolved request: request line, headers in final emission
/// order, and body bytes exactly as they will be written to the socket.
/// This is the byte-exact guarantee's boundary — nothing downstream of
/// `to_bytes()` reorders, merges, or reinterprets anything.
#[derive(Debug, Clone, PartialEq)]
pub struct RawRequest {
    pub method: String,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawRequest {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.body.len());
        out.extend_from_slice(self.method.as_bytes());
        out.push(b' ');
        out.extend_from_slice(self.target.as_bytes());
        out.extend_from_slice(b" HTTP/1.1\r\n");
        for (name, value) in &self.headers {
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(b": ");
            out.extend_from_slice(value.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

/// Resolves a [`RequestSpec`] into a [`RawRequest`]. This is where the
/// byte-exact header algorithm lives:
///
/// 1. `Host` is emitted first, immediately after the request line, unless
///    the caller's own directives already include one (in which case
///    theirs wins, in its given position, and no duplicate is added).
/// 2. The caller's directives are walked once, in order, each emitted (or
///    suppressed) verbatim — duplicates and unusual casing pass through
///    untouched.
/// 3. Any default (`Cookie`, body framing, `Content-Type`, `User-Agent`,
///    `Accept`, `Accept-Encoding`, `Connection`) not already covered by a
///    caller directive is appended after, in that fixed order.
pub fn build(spec: RequestSpec) -> RawRequest {
    let mut satisfied: HashSet<String> = HashSet::new();
    let mut headers: Vec<(String, String)> = Vec::new();

    let has_explicit_host = spec
        .directives
        .iter()
        .any(|d| d.name().eq_ignore_ascii_case("host"));
    if !has_explicit_host {
        headers.push(("Host".to_string(), spec.host_default.clone()));
    }
    satisfied.insert("host".to_string());

    for directive in &spec.directives {
        satisfied.insert(directive.name().to_ascii_lowercase());
        if let HeaderDirective::Set(name, value) = directive {
            headers.push((name.clone(), value.clone()));
        }
    }

    if !satisfied.contains("cookie")
        && let Some(cookie) = &spec.cookie_default
    {
        headers.push(("Cookie".to_string(), cookie.clone()));
    }

    let has_body = !spec.body.is_empty() || spec.force_chunked;
    let body = if spec.force_chunked {
        encode_chunked(&spec.body)
    } else {
        spec.body
    };

    if has_body {
        if spec.force_chunked {
            if !satisfied.contains("transfer-encoding") {
                headers.push(("Transfer-Encoding".to_string(), "chunked".to_string()));
            }
        } else if !satisfied.contains("content-length") {
            headers.push(("Content-Length".to_string(), body.len().to_string()));
        }
        if !satisfied.contains("content-type")
            && let Some(ct) = &spec.content_type_default
        {
            headers.push(("Content-Type".to_string(), ct.clone()));
        }
    }

    if !satisfied.contains("user-agent") {
        headers.push(("User-Agent".to_string(), spec.user_agent_default));
    }
    if !satisfied.contains("accept") {
        headers.push(("Accept".to_string(), spec.accept_default));
    }
    if !satisfied.contains("accept-encoding")
        && let Some(ae) = spec.accept_encoding_default
    {
        headers.push(("Accept-Encoding".to_string(), ae));
    }
    if !satisfied.contains("connection") {
        headers.push(("Connection".to_string(), spec.connection_default));
    }

    RawRequest {
        method: spec.method,
        target: spec.target,
        headers,
        body,
    }
}

/// Chunk-frames the entire body as a single chunk followed by the
/// zero-length final chunk. Correct per RFC 7230 §4.1; bodies are fully
/// buffered in memory in v1 rather than streamed, so there is never a
/// reason to split this into multiple chunks.
fn encode_chunked(body: &[u8]) -> Vec<u8> {
    if body.is_empty() {
        return b"0\r\n\r\n".to_vec();
    }
    let mut out = Vec::with_capacity(body.len() + 16);
    out.extend_from_slice(format!("{:x}\r\n", body.len()).as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\r\n0\r\n\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_spec() -> RequestSpec {
        RequestSpec {
            method: "GET".to_string(),
            target: "/".to_string(),
            directives: Vec::new(),
            host_default: "example.com".to_string(),
            user_agent_default: "sforge/1.0.0".to_string(),
            accept_default: "*/*".to_string(),
            accept_encoding_default: None,
            connection_default: "close".to_string(),
            cookie_default: None,
            content_type_default: None,
            body: Vec::new(),
            force_chunked: false,
        }
    }

    #[test]
    fn minimal_get_request_bytes() {
        let raw = build(base_spec());
        assert_eq!(
            raw.to_bytes(),
            b"GET / HTTP/1.1\r\n\
              Host: example.com\r\n\
              User-Agent: sforge/1.0.0\r\n\
              Accept: */*\r\n\
              Connection: close\r\n\
              \r\n"
                .to_vec()
        );
    }

    #[test]
    fn host_is_emitted_first() {
        let raw = build(base_spec());
        assert_eq!(
            raw.headers[0],
            ("Host".to_string(), "example.com".to_string())
        );
    }

    #[test]
    fn explicit_host_directive_wins_and_keeps_its_position() {
        let mut spec = base_spec();
        spec.directives
            .push(HeaderDirective::Set("X-First".to_string(), "1".to_string()));
        spec.directives.push(HeaderDirective::Set(
            "Host".to_string(),
            "override.example".to_string(),
        ));
        let raw = build(spec);

        // No auto-Host at the front; the caller's Host appears at its own
        // position in the directive list instead.
        assert_eq!(raw.headers[0], ("X-First".to_string(), "1".to_string()));
        assert_eq!(
            raw.headers[1],
            ("Host".to_string(), "override.example".to_string())
        );
        assert_eq!(raw.headers.iter().filter(|(n, _)| n == "Host").count(), 1);
    }

    #[test]
    fn duplicate_headers_are_preserved_in_order_never_merged() {
        let mut spec = base_spec();
        spec.directives.push(HeaderDirective::Set(
            "X-Test".to_string(),
            "one".to_string(),
        ));
        spec.directives.push(HeaderDirective::Set(
            "X-Test".to_string(),
            "two".to_string(),
        ));
        let raw = build(spec);

        let values: Vec<&str> = raw
            .headers
            .iter()
            .filter(|(n, _)| n == "X-Test")
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(values, vec!["one", "two"]);
    }

    #[test]
    fn suppress_directive_omits_the_header_entirely() {
        let mut spec = base_spec();
        spec.directives
            .push(HeaderDirective::Suppress("Accept".to_string()));
        let raw = build(spec);
        assert!(
            !raw.headers
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case("accept"))
        );
    }

    #[test]
    fn suppressing_host_leaves_no_host_header_at_all() {
        let mut spec = base_spec();
        spec.directives
            .push(HeaderDirective::Suppress("Host".to_string()));
        let raw = build(spec);
        assert!(
            !raw.headers
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case("host"))
        );
    }

    #[test]
    fn set_with_empty_value_sends_a_blank_header() {
        let mut spec = base_spec();
        spec.directives
            .push(HeaderDirective::Set("X-Empty".to_string(), String::new()));
        let raw = build(spec);
        assert!(
            raw.headers
                .contains(&("X-Empty".to_string(), String::new()))
        );
        assert!(raw.to_bytes().windows(10).any(|w| w == b"X-Empty: \r"));
    }

    #[test]
    fn unusual_casing_is_preserved_verbatim() {
        let mut spec = base_spec();
        spec.directives.push(HeaderDirective::Set(
            "x-CuStOm-HEADER".to_string(),
            "v".to_string(),
        ));
        let raw = build(spec);
        assert!(raw.headers.iter().any(|(n, _)| n == "x-CuStOm-HEADER"));
    }

    #[test]
    fn body_gets_content_length_and_default_content_type() {
        let mut spec = base_spec();
        spec.method = "POST".to_string();
        spec.body = b"a=1&b=2".to_vec();
        spec.content_type_default = Some("application/x-www-form-urlencoded".to_string());
        let raw = build(spec);

        assert!(
            raw.headers
                .contains(&("Content-Length".to_string(), "7".to_string()))
        );
        assert!(raw.headers.contains(&(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string()
        )));
        assert_eq!(raw.body, b"a=1&b=2");
    }

    #[test]
    fn manual_content_length_overrides_the_computed_one_even_if_wrong() {
        // Deliberately mismatched Content-Length is a real smuggling test —
        // the byte-exact engine must send exactly what was asked, not what
        // is "correct".
        let mut spec = base_spec();
        spec.method = "POST".to_string();
        spec.body = b"0123456789".to_vec();
        spec.directives.push(HeaderDirective::Set(
            "Content-Length".to_string(),
            "5".to_string(),
        ));
        let raw = build(spec);

        let lengths: Vec<&str> = raw
            .headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case("content-length"))
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(lengths, vec!["5"]);
        assert_eq!(raw.body.len(), 10);
    }

    #[test]
    fn force_chunked_frames_the_body_and_sets_transfer_encoding() {
        let mut spec = base_spec();
        spec.method = "POST".to_string();
        spec.body = b"hello".to_vec();
        spec.force_chunked = true;
        let raw = build(spec);

        assert!(
            raw.headers
                .contains(&("Transfer-Encoding".to_string(), "chunked".to_string()))
        );
        assert!(!raw.headers.iter().any(|(n, _)| n == "Content-Length"));
        assert_eq!(raw.body, b"5\r\nhello\r\n0\r\n\r\n".to_vec());
    }

    #[test]
    fn cookie_default_is_appended_when_not_overridden() {
        let mut spec = base_spec();
        spec.cookie_default = Some("a=1; b=2".to_string());
        let raw = build(spec);
        assert!(
            raw.headers
                .contains(&("Cookie".to_string(), "a=1; b=2".to_string()))
        );
    }

    #[test]
    fn explicit_cookie_directive_suppresses_the_session_default() {
        let mut spec = base_spec();
        spec.cookie_default = Some("a=1".to_string());
        spec.directives
            .push(HeaderDirective::Suppress("Cookie".to_string()));
        let raw = build(spec);
        assert!(!raw.headers.iter().any(|(n, _)| n == "Cookie"));
    }

    #[test]
    fn accept_encoding_only_appears_when_compressed_is_requested() {
        let raw_without = build(base_spec());
        assert!(
            !raw_without
                .headers
                .iter()
                .any(|(n, _)| n == "Accept-Encoding")
        );

        let mut spec = base_spec();
        spec.accept_encoding_default = Some("gzip, deflate, br".to_string());
        let raw_with = build(spec);
        assert!(raw_with.headers.contains(&(
            "Accept-Encoding".to_string(),
            "gzip, deflate, br".to_string()
        )));
    }
}
