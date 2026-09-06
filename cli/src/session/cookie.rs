use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::{Error, Result};

/// The exact HTTP-date format used for `Expires=` in both directions of the
/// setup-command contract: parsed here, and produced by the browser
/// extension when it builds `--set-cookie` fragments.
const HTTP_DATE: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<SameSite>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub expires: Option<OffsetDateTime>,
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl CookieEntry {
    /// Parses a `--set-cookie` value using real Set-Cookie attribute syntax:
    /// `name=value; Domain=...; Path=...; Secure; HttpOnly; SameSite=Lax; Expires=...`.
    ///
    /// `default_domain` is used when no `Domain=` attribute is present; when
    /// `None` (the `session setup` case, which has no request host to fall
    /// back to), a missing `Domain=` is an error.
    pub fn parse_set_cookie(raw: &str, default_domain: Option<&str>) -> Result<Self> {
        let mut parts = raw.split(';').map(str::trim);
        let pair = parts
            .next()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| Error::InvalidSetCookie(raw.to_string()))?;
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| Error::InvalidSetCookie(raw.to_string()))?;
        if name.is_empty() {
            return Err(Error::InvalidSetCookie(raw.to_string()));
        }

        let mut domain = default_domain.map(str::to_string);
        let mut path = default_path();
        let mut secure = false;
        let mut http_only = false;
        let mut same_site = None;
        let mut expires = None;

        for attr in parts {
            if attr.is_empty() {
                continue;
            }
            let (key, val) = attr
                .split_once('=')
                .map(|(k, v)| (k, Some(v.trim())))
                .unwrap_or((attr, None));

            match key.to_ascii_lowercase().as_str() {
                "domain" => {
                    domain = val
                        .filter(|v| !v.is_empty())
                        .map(|v| v.trim_start_matches('.').to_string())
                }
                "path" => path = val.filter(|v| !v.is_empty()).unwrap_or("/").to_string(),
                "secure" => secure = true,
                "httponly" => http_only = true,
                "samesite" => {
                    same_site = match val.unwrap_or_default().to_ascii_lowercase().as_str() {
                        "strict" => Some(SameSite::Strict),
                        "lax" => Some(SameSite::Lax),
                        "none" => Some(SameSite::None),
                        _ => None,
                    }
                }
                "expires" => {
                    if let Some(v) = val {
                        // HTTP-date is always GMT, so parse as a naive
                        // datetime and attach the UTC offset ourselves —
                        // OffsetDateTime::parse requires an offset component
                        // in the format string, which a literal "GMT" isn't.
                        expires = time::PrimitiveDateTime::parse(v, HTTP_DATE)
                            .ok()
                            .map(time::PrimitiveDateTime::assume_utc);
                    }
                }
                "max-age" => {
                    if let Some(seconds) = val.and_then(|v| v.parse::<i64>().ok()) {
                        expires =
                            Some(OffsetDateTime::now_utc() + time::Duration::seconds(seconds));
                    }
                }
                _ => {}
            }
        }

        let domain = domain
            .ok_or_else(|| Error::InvalidSetCookie(format!("{raw} (missing Domain attribute)")))?;

        Ok(Self {
            name: name.to_string(),
            value: value.to_string(),
            domain,
            path,
            secure,
            http_only,
            same_site,
            expires,
        })
    }

    /// Renders this cookie back into `--set-cookie` syntax, primarily for
    /// display (`session` TUI detail pane, `--trace`).
    pub fn to_set_cookie_string(&self) -> String {
        let mut out = format!(
            "{}={}; Domain={}; Path={}",
            self.name, self.value, self.domain, self.path
        );
        if self.secure {
            out.push_str("; Secure");
        }
        if self.http_only {
            out.push_str("; HttpOnly");
        }
        match self.same_site {
            Some(SameSite::Strict) => out.push_str("; SameSite=Strict"),
            Some(SameSite::Lax) => out.push_str("; SameSite=Lax"),
            Some(SameSite::None) => out.push_str("; SameSite=None"),
            None => {}
        }
        if let Some(expires) = self.expires
            && let Ok(formatted) = expires.format(HTTP_DATE)
        {
            out.push_str("; Expires=");
            out.push_str(&formatted);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_cookie_with_default_domain() {
        let cookie =
            CookieEntry::parse_set_cookie("session_id=abc123", Some("target.com")).unwrap();
        assert_eq!(cookie.name, "session_id");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.domain, "target.com");
        assert_eq!(cookie.path, "/");
        assert!(!cookie.secure);
        assert!(!cookie.http_only);
        assert_eq!(cookie.same_site, None);
        assert_eq!(cookie.expires, None);
    }

    #[test]
    fn missing_domain_without_default_is_an_error() {
        let result = CookieEntry::parse_set_cookie("a=b", None);
        assert!(result.is_err());
    }

    #[test]
    fn parses_full_fidelity_attributes() {
        let raw =
            "csrf=xyz789; Domain=.app.target.com; Path=/api; Secure; HttpOnly; SameSite=Strict";
        let cookie = CookieEntry::parse_set_cookie(raw, None).unwrap();
        assert_eq!(cookie.name, "csrf");
        assert_eq!(cookie.value, "xyz789");
        assert_eq!(cookie.domain, "app.target.com");
        assert_eq!(cookie.path, "/api");
        assert!(cookie.secure);
        assert!(cookie.http_only);
        assert_eq!(cookie.same_site, Some(SameSite::Strict));
    }

    #[test]
    fn trailing_semicolon_and_blank_attributes_are_ignored() {
        let cookie = CookieEntry::parse_set_cookie("a=b; Domain=x.test; ;", None).unwrap();
        assert_eq!(cookie.domain, "x.test");
    }

    #[test]
    fn max_age_sets_a_concrete_expiry_relative_to_now() {
        let cookie =
            CookieEntry::parse_set_cookie("a=b; Domain=x.test; Max-Age=3600", None).unwrap();
        let expires = cookie.expires.expect("expires should be set from Max-Age");
        let delta = expires - OffsetDateTime::now_utc();
        assert!(delta.whole_seconds() > 3590 && delta.whole_seconds() <= 3600);
    }

    #[test]
    fn expires_attribute_round_trips_through_http_date() {
        let raw = "a=b; Domain=x.test; Expires=Wed, 21 Oct 2026 07:28:00 GMT";
        let cookie = CookieEntry::parse_set_cookie(raw, None).unwrap();
        let expires = cookie.expires.expect("expires should parse");
        assert_eq!(expires.year(), 2026);
        assert_eq!(
            cookie.to_set_cookie_string(),
            "a=b; Domain=x.test; Path=/; Expires=Wed, 21 Oct 2026 07:28:00 GMT"
        );
    }

    #[test]
    fn unrecognized_attributes_are_ignored_not_fatal() {
        let cookie =
            CookieEntry::parse_set_cookie("a=b; Domain=x.test; Priority=High", None).unwrap();
        assert_eq!(cookie.domain, "x.test");
    }
}
