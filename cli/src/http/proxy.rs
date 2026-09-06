use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::auth::base64_encode;
use crate::error::{Error, Result};
use crate::http::connection::Connection;

#[derive(Debug, Clone)]
pub struct ProxyTarget {
    pub host: String,
    pub port: u16,
    pub userinfo: Option<(String, Option<String>)>,
}

/// A `Proxy-Authorization` header to attach to the *actual* request, used
/// only for the plain-HTTP-through-proxy relay case — the `CONNECT` tunnel
/// case sends its own `Proxy-Authorization` as part of the tunnel setup
/// and needs nothing added to the tunneled request itself.
pub fn proxy_authorization(target: &ProxyTarget) -> Option<(String, String)> {
    let (user, pass) = target.userinfo.as_ref()?;
    let credentials =
        base64_encode(format!("{user}:{}", pass.clone().unwrap_or_default()).as_bytes());
    Some((
        "Proxy-Authorization".to_string(),
        format!("Basic {credentials}"),
    ))
}

/// Connects to `proxy`, issues `CONNECT target_host:target_port`, and
/// returns the raw tunnel once the proxy answers `200` — ready for a TLS
/// handshake to run through it exactly as if it dialed the origin directly.
pub fn connect_tunnel(
    proxy: &ProxyTarget,
    target_host: &str,
    target_port: u16,
    timeout: Option<Duration>,
) -> Result<TcpStream> {
    let mut stream = Connection::connect_plain(&proxy.host, proxy.port, timeout)?;
    let proxy_addr = format!("{}:{}", proxy.host, proxy.port);

    let mut request = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n"
    );
    if let Some((name, value)) = proxy_authorization(proxy) {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .map_err(|source| Error::Io {
            path: proxy_addr.clone().into(),
            source,
        })?;
    stream.flush().map_err(|source| Error::Io {
        path: proxy_addr.clone().into(),
        source,
    })?;

    let mut reader = BufReader::new(&mut stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|source| Error::Io {
            path: proxy_addr.clone().into(),
            source,
        })?;

    let status_ok = status_line
        .split_whitespace()
        .nth(1)
        .is_some_and(|code| code == "200");
    if !status_ok {
        return Err(Error::Connect(format!(
            "proxy CONNECT to {target_host}:{target_port} failed: {}",
            status_line.trim()
        )));
    }

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|source| Error::Io {
            path: proxy_addr.clone().into(),
            source,
        })?;
        if line.trim().is_empty() {
            break;
        }
    }

    drop(reader);
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_authorization_without_credentials() {
        let target = ProxyTarget {
            host: "proxy.local".to_string(),
            port: 8080,
            userinfo: None,
        };
        assert!(proxy_authorization(&target).is_none());
    }

    #[test]
    fn builds_basic_proxy_authorization() {
        let target = ProxyTarget {
            host: "proxy.local".to_string(),
            port: 8080,
            userinfo: Some(("user".to_string(), Some("pass".to_string()))),
        };
        let (name, value) = proxy_authorization(&target).unwrap();
        assert_eq!(name, "Proxy-Authorization");
        assert_eq!(value, format!("Basic {}", base64_encode(b"user:pass")));
    }
}
