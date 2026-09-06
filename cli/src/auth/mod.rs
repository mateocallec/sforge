pub mod digest;

use crate::http::request_writer::HeaderDirective;
use crate::session::{Auth, AuthKind};

/// What to do about auth on the *first* attempt of a request: Basic and
/// Bearer can be computed immediately from what's stored. Digest cannot —
/// it needs a server-issued nonce first — so it's surfaced separately for
/// the caller to retry once a 401 challenge has been seen.
pub enum AuthPlan {
    None,
    Immediate(HeaderDirective),
    NeedsChallenge,
}

pub fn plan(auth: &Auth) -> AuthPlan {
    let Some(kind) = auth.kind else {
        return AuthPlan::None;
    };

    match kind {
        AuthKind::Basic => {
            let username = auth.username.clone().unwrap_or_default();
            let password = auth.password.clone().unwrap_or_default();
            let encoded = base64_encode(format!("{username}:{password}").as_bytes());
            AuthPlan::Immediate(HeaderDirective::Set(
                "Authorization".to_string(),
                format!("Basic {encoded}"),
            ))
        }
        AuthKind::Bearer => {
            let token = auth.password.clone().unwrap_or_default();
            AuthPlan::Immediate(HeaderDirective::Set(
                "Authorization".to_string(),
                format!("Bearer {token}"),
            ))
        }
        AuthKind::Digest => AuthPlan::NeedsChallenge,
    }
}

/// A fresh per-request client nonce for Digest auth. A v4 UUID's hex form
/// is a perfectly valid opaque cnonce and avoids pulling in a dedicated
/// RNG crate just for this.
pub fn fresh_cnonce() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 (RFC 4648 §4), encode-only — the only direction
/// `Authorization: Basic` / `Proxy-Authorization: Basic` ever need.
pub fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();

        out.push(BASE64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(BASE64_ALPHABET[(((b0 & 0x03) << 4) | (b1.unwrap_or(0) >> 4)) as usize] as char);
        out.push(match b1 {
            Some(b1) => {
                BASE64_ALPHABET[(((b1 & 0x0f) << 2) | (b2.unwrap_or(0) >> 6)) as usize] as char
            }
            None => '=',
        });
        out.push(match b2 {
            Some(b2) => BASE64_ALPHABET[(b2 & 0x3f) as usize] as char,
            None => '=',
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn basic_plan_encodes_user_and_pass() {
        let auth = Auth {
            kind: Some(AuthKind::Basic),
            username: Some("admin".to_string()),
            password: Some("s3cr3t".to_string()),
        };
        match plan(&auth) {
            AuthPlan::Immediate(HeaderDirective::Set(name, value)) => {
                assert_eq!(name, "Authorization");
                assert_eq!(value, format!("Basic {}", base64_encode(b"admin:s3cr3t")));
            }
            _ => panic!("expected an immediate Basic header"),
        }
    }

    #[test]
    fn bearer_plan_uses_the_token_directly() {
        let auth = Auth {
            kind: Some(AuthKind::Bearer),
            username: None,
            password: Some("token123".to_string()),
        };
        match plan(&auth) {
            AuthPlan::Immediate(HeaderDirective::Set(name, value)) => {
                assert_eq!(name, "Authorization");
                assert_eq!(value, "Bearer token123");
            }
            _ => panic!("expected an immediate Bearer header"),
        }
    }

    #[test]
    fn digest_plan_needs_a_challenge_first() {
        let auth = Auth {
            kind: Some(AuthKind::Digest),
            username: Some("u".to_string()),
            password: Some("p".to_string()),
        };
        assert!(matches!(plan(&auth), AuthPlan::NeedsChallenge));
    }

    #[test]
    fn no_auth_kind_means_no_header() {
        assert!(matches!(plan(&Auth::default()), AuthPlan::None));
    }
}
