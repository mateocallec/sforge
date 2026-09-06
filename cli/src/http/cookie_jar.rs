use crate::http::url::{RequestUrl, Scheme};
use crate::session::cookie::CookieEntry;

/// Selects the cookies from `cookies` that apply to `url` per standard
/// cookie-matching rules (RFC 6265 §5.1.3/§5.1.4): domain match (exact, or
/// a subdomain of a `Domain`-scoped cookie), path-prefix match, and
/// `Secure` cookies sent only over HTTPS. Input order is preserved, since
/// a session's cookies may span several domains and this is what lets a
/// redirect across hosts recompute the right subset for each hop.
pub fn matching<'a>(cookies: &'a [CookieEntry], url: &RequestUrl) -> Vec<&'a CookieEntry> {
    cookies
        .iter()
        .filter(|c| {
            domain_matches(&c.domain, &url.host)
                && path_matches(&c.path, &url.raw_target)
                && (!c.secure || url.scheme == Scheme::Https)
        })
        .collect()
}

/// Joins matching cookies into one `Cookie:` header value
/// (`name=value; name2=value2`); `None` if nothing matches.
pub fn header_value(cookies: &[CookieEntry], url: &RequestUrl) -> Option<String> {
    let matched = matching(cookies, url);
    if matched.is_empty() {
        return None;
    }
    Some(
        matched
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn domain_matches(cookie_domain: &str, request_host: &str) -> bool {
    let cookie_domain = cookie_domain.trim_start_matches('.').to_ascii_lowercase();
    let request_host = request_host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    request_host == cookie_domain || request_host.ends_with(&format!(".{cookie_domain}"))
}

fn path_matches(cookie_path: &str, request_target: &str) -> bool {
    let request_path = request_target.split('?').next().unwrap_or("/");
    if request_path == cookie_path {
        return true;
    }
    match request_path.strip_prefix(cookie_path) {
        Some(rest) => cookie_path.ends_with('/') || rest.starts_with('/'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cookie(name: &str, domain: &str, path: &str, secure: bool) -> CookieEntry {
        CookieEntry {
            name: name.to_string(),
            value: "v".to_string(),
            domain: domain.to_string(),
            path: path.to_string(),
            secure,
            http_only: false,
            same_site: None,
            expires: None,
        }
    }

    #[test]
    fn matches_exact_domain() {
        let cookies = vec![cookie("a", "example.com", "/", false)];
        let url = RequestUrl::parse("https://example.com/").unwrap();
        assert_eq!(header_value(&cookies, &url), Some("a=v".to_string()));
    }

    #[test]
    fn matches_subdomain_of_a_domain_scoped_cookie() {
        let cookies = vec![cookie("a", "target.com", "/", false)];
        let url = RequestUrl::parse("https://api.target.com/").unwrap();
        assert_eq!(header_value(&cookies, &url), Some("a=v".to_string()));
    }

    #[test]
    fn does_not_match_unrelated_domain() {
        let cookies = vec![cookie("a", "target.com", "/", false)];
        let url = RequestUrl::parse("https://evil.com/").unwrap();
        assert_eq!(header_value(&cookies, &url), None);
    }

    #[test]
    fn does_not_treat_suffix_lookalike_as_a_subdomain() {
        let cookies = vec![cookie("a", "target.com", "/", false)];
        let url = RequestUrl::parse("https://nottarget.com/").unwrap();
        assert_eq!(header_value(&cookies, &url), None);
    }

    #[test]
    fn respects_path_prefix() {
        let cookies = vec![cookie("a", "example.com", "/admin", false)];
        let inside = RequestUrl::parse("https://example.com/admin/users").unwrap();
        let outside = RequestUrl::parse("https://example.com/public").unwrap();
        assert_eq!(header_value(&cookies, &inside), Some("a=v".to_string()));
        assert_eq!(header_value(&cookies, &outside), None);
    }

    #[test]
    fn secure_cookie_only_sent_over_https() {
        let cookies = vec![cookie("a", "example.com", "/", true)];
        let https = RequestUrl::parse("https://example.com/").unwrap();
        let http = RequestUrl::parse("http://example.com/").unwrap();
        assert_eq!(header_value(&cookies, &https), Some("a=v".to_string()));
        assert_eq!(header_value(&cookies, &http), None);
    }

    #[test]
    fn joins_multiple_matches_in_input_order() {
        let cookies = vec![
            cookie("a", "example.com", "/", false),
            cookie("b", "example.com", "/", false),
        ];
        let url = RequestUrl::parse("https://example.com/").unwrap();
        assert_eq!(header_value(&cookies, &url), Some("a=v; b=v".to_string()));
    }
}
