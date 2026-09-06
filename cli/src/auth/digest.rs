use std::collections::HashMap;

use md5::Md5;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Md5,
    Sha256,
}

#[derive(Debug, Clone)]
pub struct DigestChallenge {
    pub realm: String,
    pub nonce: String,
    pub qop: Option<String>,
    pub opaque: Option<String>,
    pub algorithm: Algorithm,
}

impl DigestChallenge {
    /// Parses a `WWW-Authenticate: Digest ...` header value.
    pub fn parse(header_value: &str) -> Option<Self> {
        let rest = header_value.trim();
        let rest = rest
            .strip_prefix("Digest")
            .or_else(|| rest.strip_prefix("digest"))?
            .trim();

        let params = parse_auth_params(rest);
        let realm = params.get("realm")?.clone();
        let nonce = params.get("nonce")?.clone();
        let qop = params.get("qop").cloned();
        let opaque = params.get("opaque").cloned();
        let algorithm = match params.get("algorithm").map(String::as_str) {
            Some("SHA-256") => Algorithm::Sha256,
            _ => Algorithm::Md5,
        };

        Some(Self {
            realm,
            nonce,
            qop,
            opaque,
            algorithm,
        })
    }
}

fn parse_auth_params(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in split_unquoted_commas(s) {
        if let Some((key, value)) = part.split_once('=') {
            map.insert(
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            );
        }
    }
    map
}

/// Splits on commas that aren't inside a double-quoted value — several
/// Digest parameters (notably `qop`) are themselves comma-separated lists
/// wrapped in one quoted string.
fn split_unquoted_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_quotes = false;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(s[start..].trim());
    parts
}

pub struct DigestResponse {
    pub header_value: String,
}

/// Computes an RFC 7616 Digest `Authorization` header for one request,
/// given the server's challenge. Scoped to `qop=auth` (not `auth-int`,
/// which additionally hashes the request body — rare enough in practice to
/// leave for a later version) with MD5 or SHA-256, matching whichever
/// `algorithm` the challenge specified.
pub fn compute(
    challenge: &DigestChallenge,
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    cnonce: &str,
    nc: u32,
) -> DigestResponse {
    let nc_str = format!("{nc:08x}");
    let qop_value = challenge
        .qop
        .as_deref()
        .map(|qop| qop.split(',').next().unwrap_or("auth").trim());

    let ha1 = hash(
        challenge.algorithm,
        &format!("{username}:{}:{password}", challenge.realm),
    );
    let ha2 = hash(challenge.algorithm, &format!("{method}:{uri}"));

    let response = match qop_value {
        Some(qop) => hash(
            challenge.algorithm,
            &format!("{ha1}:{}:{nc_str}:{cnonce}:{qop}:{ha2}", challenge.nonce),
        ),
        None => hash(
            challenge.algorithm,
            &format!("{ha1}:{}:{ha2}", challenge.nonce),
        ),
    };

    let mut header = format!(
        "Digest username=\"{username}\", realm=\"{}\", nonce=\"{}\", uri=\"{uri}\", response=\"{response}\"",
        challenge.realm, challenge.nonce
    );
    if let Some(opaque) = &challenge.opaque {
        header.push_str(&format!(", opaque=\"{opaque}\""));
    }
    if let Some(qop) = qop_value {
        header.push_str(&format!(", qop={qop}, nc={nc_str}, cnonce=\"{cnonce}\""));
    }
    if challenge.algorithm == Algorithm::Sha256 {
        header.push_str(", algorithm=SHA-256");
    }

    DigestResponse {
        header_value: header,
    }
}

fn hash(algorithm: Algorithm, input: &str) -> String {
    match algorithm {
        Algorithm::Md5 => {
            let mut hasher = Md5::new();
            hasher.update(input.as_bytes());
            hex(&hasher.finalize())
        }
        Algorithm::Sha256 => {
            let mut hasher = Sha256::new();
            hasher.update(input.as_bytes());
            hex(&hasher.finalize())
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc2617_worked_example() {
        // The canonical worked example from RFC 2617 §3.5.
        let challenge = DigestChallenge {
            realm: "testrealm@host.com".to_string(),
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
            qop: Some("auth".to_string()),
            opaque: Some("5ccc069c403ebaf9f0171e9517f40e41".to_string()),
            algorithm: Algorithm::Md5,
        };

        let response = compute(
            &challenge,
            "Mufasa",
            "Circle Of Life",
            "GET",
            "/dir/index.html",
            "0a4f113b",
            1,
        );

        assert!(
            response
                .header_value
                .contains("response=\"6629fae49393a05397450978507c4ef1\"")
        );
    }

    #[test]
    fn parses_challenge_with_quoted_qop_list() {
        let header = r#"Digest realm="test", qop="auth,auth-int", nonce="abc123", opaque="xyz""#;
        let challenge = DigestChallenge::parse(header).unwrap();
        assert_eq!(challenge.realm, "test");
        assert_eq!(challenge.nonce, "abc123");
        assert_eq!(challenge.qop.as_deref(), Some("auth,auth-int"));
        assert_eq!(challenge.opaque.as_deref(), Some("xyz"));
        assert_eq!(challenge.algorithm, Algorithm::Md5);
    }

    #[test]
    fn parses_sha256_algorithm() {
        let header = r#"Digest realm="test", nonce="abc", algorithm=SHA-256"#;
        let challenge = DigestChallenge::parse(header).unwrap();
        assert_eq!(challenge.algorithm, Algorithm::Sha256);
    }

    #[test]
    fn returns_none_for_non_digest_scheme() {
        assert!(DigestChallenge::parse(r#"Basic realm="test""#).is_none());
    }

    #[test]
    fn works_without_qop() {
        let challenge = DigestChallenge {
            realm: "test".to_string(),
            nonce: "abc".to_string(),
            qop: None,
            opaque: None,
            algorithm: Algorithm::Md5,
        };
        let response = compute(&challenge, "u", "p", "GET", "/", "cnonce", 1);
        assert!(!response.header_value.contains("qop="));
        assert!(response.header_value.contains("response=\""));
    }
}
