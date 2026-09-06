use crate::http::url::{RequestUrl, Scheme};

/// What happens to the method/body when following a redirect of a given
/// status code (RFC 7231 §6.4 / RFC 7238), returning `None` for anything
/// that isn't itself a redirect status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyPolicy {
    /// 301/302/303: switch to GET and drop the body — universal real-world
    /// browser/curl behavior even though 301/302 are technically ambiguous
    /// in the original spec.
    SwitchToGet,
    /// 307/308: method and body are preserved exactly.
    Preserve,
}

pub fn body_policy_for(status_code: u16) -> Option<BodyPolicy> {
    match status_code {
        301..=303 => Some(BodyPolicy::SwitchToGet),
        307 | 308 => Some(BodyPolicy::Preserve),
        _ => None,
    }
}

pub fn is_redirect(status_code: u16) -> bool {
    body_policy_for(status_code).is_some()
}

/// Resolves a `Location` header value against the URL that was just
/// requested. Handles an absolute URL, a scheme-relative URL (`//host/…`),
/// an absolute path, and a same-directory relative path. Full RFC 3986
/// dot-segment resolution for relative paths is not implemented — those
/// are rare in real-world `Location` headers, which are overwhelmingly
/// absolute paths or absolute URLs.
pub fn resolve_location(current: &RequestUrl, location: &str) -> Option<RequestUrl> {
    let location = location.trim();
    if location.is_empty() {
        return None;
    }

    if location.contains("://") {
        return RequestUrl::parse(location).ok();
    }

    let scheme_str = match current.scheme {
        Scheme::Http => "http",
        Scheme::Https => "https",
    };

    if let Some(rest) = location.strip_prefix("//") {
        return RequestUrl::parse(&format!("{scheme_str}://{rest}")).ok();
    }

    let authority = current.default_host_header();

    let path = if location.starts_with('/') {
        location.to_string()
    } else {
        let base_dir = current
            .raw_target
            .split('?')
            .next()
            .unwrap_or("/")
            .rsplit_once('/')
            .map(|(dir, _)| dir)
            .unwrap_or("");
        format!("{base_dir}/{location}")
    };

    RequestUrl::parse(&format!("{scheme_str}://{authority}{path}")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_to_get_for_301_302_303() {
        assert_eq!(body_policy_for(301), Some(BodyPolicy::SwitchToGet));
        assert_eq!(body_policy_for(302), Some(BodyPolicy::SwitchToGet));
        assert_eq!(body_policy_for(303), Some(BodyPolicy::SwitchToGet));
    }

    #[test]
    fn preserve_for_307_308() {
        assert_eq!(body_policy_for(307), Some(BodyPolicy::Preserve));
        assert_eq!(body_policy_for(308), Some(BodyPolicy::Preserve));
    }

    #[test]
    fn not_a_redirect_for_other_statuses() {
        assert_eq!(body_policy_for(200), None);
        assert_eq!(body_policy_for(404), None);
    }

    #[test]
    fn resolves_absolute_url() {
        let current = RequestUrl::parse("https://a.test/old").unwrap();
        let next = resolve_location(&current, "https://b.test/new").unwrap();
        assert_eq!(next.host, "b.test");
        assert_eq!(next.raw_target, "/new");
    }

    #[test]
    fn resolves_absolute_path_on_same_host() {
        let current = RequestUrl::parse("https://a.test/old/page?x=1").unwrap();
        let next = resolve_location(&current, "/new/path").unwrap();
        assert_eq!(next.host, "a.test");
        assert_eq!(next.raw_target, "/new/path");
    }

    #[test]
    fn resolves_scheme_relative_url() {
        let current = RequestUrl::parse("https://a.test/old").unwrap();
        let next = resolve_location(&current, "//cdn.test/asset").unwrap();
        assert_eq!(next.scheme, current.scheme);
        assert_eq!(next.host, "cdn.test");
    }

    #[test]
    fn resolves_relative_path_against_current_directory() {
        let current = RequestUrl::parse("https://a.test/dir/page").unwrap();
        let next = resolve_location(&current, "sibling").unwrap();
        assert_eq!(next.raw_target, "/dir/sibling");
    }

    #[test]
    fn preserves_explicit_port_across_a_path_redirect() {
        let current = RequestUrl::parse("https://a.test:8443/old").unwrap();
        let next = resolve_location(&current, "/new").unwrap();
        assert_eq!(next.port, 8443);
    }
}
