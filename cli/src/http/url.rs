use crate::error::{Error, Result};

/// Hand-rolled rather than delegated to the `url` crate: the one thing that
/// actually matters here — the raw path+query — must survive completely
/// unnormalized (no dot-segment collapsing, no percent-encoding
/// canonicalization), which a general-purpose URL parser is specifically
/// designed to prevent. IDN (non-ASCII) hostnames are out of scope for v1;
/// pentest targets are overwhelmingly ASCII hostnames or bare IPs, and
/// `--resolve`/`--connect-to` remain available as a workaround.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    pub fn default_port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestUrl {
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    /// Credentials embedded in the URL itself (`https://user:pass@host/`),
    /// usable as a Basic-auth fallback when no explicit auth is given.
    pub userinfo: Option<(String, Option<String>)>,
    /// Path + query, taken verbatim from the input. Never empty — an
    /// absent path is stored as "/". The fragment, if any, is dropped
    /// (fragments are a client-local concept and are never sent to a
    /// server by any real HTTP client).
    pub raw_target: String,
}

impl RequestUrl {
    pub fn parse(raw: &str) -> Result<Self> {
        let (scheme_str, rest) = raw
            .split_once("://")
            .ok_or_else(|| Error::InvalidUrl(raw.to_string()))?;

        let scheme = match scheme_str.to_ascii_lowercase().as_str() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            other => return Err(Error::UnsupportedScheme(other.to_string())),
        };

        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let remainder = &rest[authority_end..];

        let raw_target_pre = remainder.split('#').next().unwrap_or("");
        let raw_target = if raw_target_pre.is_empty() {
            "/".to_string()
        } else {
            raw_target_pre.to_string()
        };

        if authority.is_empty() {
            return Err(Error::InvalidUrl(raw.to_string()));
        }

        let (userinfo_str, host_port) = match authority.rsplit_once('@') {
            Some((info, hp)) => (Some(info), hp),
            None => (None, authority),
        };

        let userinfo = userinfo_str.map(|info| match info.split_once(':') {
            Some((u, p)) => (percent_decode(u), Some(percent_decode(p))),
            None => (percent_decode(info), None),
        });

        let (host, port) = parse_host_port(host_port, scheme.default_port())
            .ok_or_else(|| Error::InvalidUrl(raw.to_string()))?;

        if host.is_empty() {
            return Err(Error::InvalidUrl(raw.to_string()));
        }

        Ok(Self {
            scheme,
            host,
            port,
            userinfo,
            raw_target,
        })
    }

    /// The default `Host` header value / TLS SNI name: the port is
    /// included only when it differs from the scheme's default, matching
    /// curl's own behavior.
    pub fn default_host_header(&self) -> String {
        if self.port == self.scheme.default_port() {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_host_port(host_port: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = host_port.strip_prefix('[') {
        let close = rest.find(']')?;
        let host = format!("[{}]", &rest[..close]);
        let after = &rest[close + 1..];
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None => default_port,
        };
        return Some((host, port));
    }

    match host_port.rsplit_once(':') {
        Some((host, port_str))
            if !port_str.is_empty() && port_str.bytes().all(|b| b.is_ascii_digit()) =>
        {
            Some((host.to_string(), port_str.parse().ok()?))
        }
        _ => Some((host_port.to_string(), default_port)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_https_url() {
        let url = RequestUrl::parse("https://example.com/path?x=1").unwrap();
        assert_eq!(url.scheme, Scheme::Https);
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 443);
        assert_eq!(url.raw_target, "/path?x=1");
    }

    #[test]
    fn defaults_empty_path_to_slash() {
        let url = RequestUrl::parse("http://example.com").unwrap();
        assert_eq!(url.raw_target, "/");
    }

    #[test]
    fn preserves_non_normalized_dot_segments_verbatim() {
        let url = RequestUrl::parse("http://example.com/foo/../../etc/passwd").unwrap();
        assert_eq!(url.raw_target, "/foo/../../etc/passwd");
    }

    #[test]
    fn preserves_percent_encoded_path_verbatim() {
        let url = RequestUrl::parse("http://example.com/foo%2f..%2fbar").unwrap();
        assert_eq!(url.raw_target, "/foo%2f..%2fbar");
    }

    #[test]
    fn strips_fragment_but_keeps_query() {
        let url = RequestUrl::parse("http://example.com/path?x=1#section").unwrap();
        assert_eq!(url.raw_target, "/path?x=1");
    }

    #[test]
    fn parses_explicit_port() {
        let url = RequestUrl::parse("http://example.com:8080/").unwrap();
        assert_eq!(url.port, 8080);
        assert_eq!(url.default_host_header(), "example.com:8080");
    }

    #[test]
    fn default_port_omitted_from_host_header() {
        let url = RequestUrl::parse("https://example.com/").unwrap();
        assert_eq!(url.default_host_header(), "example.com");
    }

    #[test]
    fn parses_ipv6_literal_with_port() {
        let url = RequestUrl::parse("http://[::1]:8080/").unwrap();
        assert_eq!(url.host, "[::1]");
        assert_eq!(url.port, 8080);
    }

    #[test]
    fn parses_ipv6_literal_without_port() {
        let url = RequestUrl::parse("https://[2001:db8::1]/path").unwrap();
        assert_eq!(url.host, "[2001:db8::1]");
        assert_eq!(url.port, 443);
    }

    #[test]
    fn parses_userinfo_credentials() {
        let url = RequestUrl::parse("https://admin:s3cr3t@example.com/").unwrap();
        assert_eq!(
            url.userinfo,
            Some(("admin".to_string(), Some("s3cr3t".to_string())))
        );
    }

    #[test]
    fn rejects_unsupported_scheme() {
        let result = RequestUrl::parse("ftp://example.com/");
        assert!(matches!(result, Err(Error::UnsupportedScheme(_))));
    }

    #[test]
    fn rejects_missing_scheme_separator() {
        assert!(RequestUrl::parse("example.com/path").is_err());
    }

    #[test]
    fn rejects_empty_authority() {
        assert!(RequestUrl::parse("http:///path").is_err());
    }
}
