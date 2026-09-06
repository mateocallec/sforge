use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::error::{Error, Result};
use crate::http::connection::Connection;

/// curl's own `socks5://` vs `socks5h://` distinction, preserved: with
/// `Client`, this host resolves the target hostname itself and sends the
/// proxy a raw IP; with `Proxy`, the hostname is sent as-is and the proxy
/// resolves it — useful when the target is only reachable via DNS the
/// proxy (not this host) can see, e.g. from a pentest jump host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveMode {
    Client,
    Proxy,
}

#[allow(clippy::too_many_arguments)]
pub fn connect(
    proxy_host: &str,
    proxy_port: u16,
    credentials: Option<&(String, Option<String>)>,
    target_host: &str,
    target_port: u16,
    resolve_mode: ResolveMode,
    timeout: Option<Duration>,
) -> Result<TcpStream> {
    let mut stream = Connection::connect_plain(proxy_host, proxy_port, timeout)?;

    let methods: &[u8] = if credentials.is_some() {
        &[0x00, 0x02]
    } else {
        &[0x00]
    };
    let mut greeting = vec![0x05, methods.len() as u8];
    greeting.extend_from_slice(methods);
    write_all(&mut stream, &greeting)?;

    let mut chosen = [0u8; 2];
    read_exact(&mut stream, &mut chosen)?;
    if chosen[0] != 0x05 {
        return Err(Error::Connect(
            "proxy did not respond with SOCKS version 5".to_string(),
        ));
    }
    match chosen[1] {
        0x00 => {}
        0x02 => negotiate_userpass(&mut stream, credentials)?,
        0xFF => {
            return Err(Error::Connect(
                "proxy rejected all offered SOCKS5 authentication methods".to_string(),
            ));
        }
        other => {
            return Err(Error::Connect(format!(
                "unsupported SOCKS5 auth method: {other}"
            )));
        }
    }

    let mut request = vec![0x05, 0x01, 0x00];
    match resolve_mode {
        ResolveMode::Proxy => {
            request.push(0x03);
            request.push(target_host.len() as u8);
            request.extend_from_slice(target_host.as_bytes());
        }
        ResolveMode::Client => match resolve_ip(target_host)? {
            IpAddr::V4(v4) => {
                request.push(0x01);
                request.extend_from_slice(&v4.octets());
            }
            IpAddr::V6(v6) => {
                request.push(0x04);
                request.extend_from_slice(&v6.octets());
            }
        },
    }
    request.extend_from_slice(&target_port.to_be_bytes());
    write_all(&mut stream, &request)?;

    let mut reply_header = [0u8; 4];
    read_exact(&mut stream, &mut reply_header)?;
    if reply_header[1] != 0x00 {
        return Err(Error::Connect(format!(
            "SOCKS5 CONNECT to {target_host}:{target_port} failed ({})",
            reply_reason(reply_header[1])
        )));
    }

    // Consume the bound-address portion of the reply; its contents are
    // never used (BND.ADDR/BND.PORT describe the proxy's outbound socket,
    // not anything the client needs to act on for a simple CONNECT).
    match reply_header[3] {
        0x01 => discard(&mut stream, 4 + 2)?,
        0x03 => {
            let mut len = [0u8; 1];
            read_exact(&mut stream, &mut len)?;
            discard(&mut stream, len[0] as usize + 2)?;
        }
        0x04 => discard(&mut stream, 16 + 2)?,
        other => {
            return Err(Error::Connect(format!(
                "unknown SOCKS5 address type: {other}"
            )));
        }
    }

    Ok(stream)
}

fn negotiate_userpass(
    stream: &mut TcpStream,
    credentials: Option<&(String, Option<String>)>,
) -> Result<()> {
    let (user, pass) = credentials
        .ok_or_else(|| Error::Connect("proxy requires SOCKS5 credentials".to_string()))?;
    let pass = pass.clone().unwrap_or_default();

    let mut request = vec![0x01, user.len() as u8];
    request.extend_from_slice(user.as_bytes());
    request.push(pass.len() as u8);
    request.extend_from_slice(pass.as_bytes());
    write_all(stream, &request)?;

    let mut response = [0u8; 2];
    read_exact(stream, &mut response)?;
    if response[1] != 0x00 {
        return Err(Error::Connect("SOCKS5 authentication failed".to_string()));
    }
    Ok(())
}

fn resolve_ip(host: &str) -> Result<IpAddr> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse() {
        return Ok(ip);
    }
    (host, 0u16)
        .to_socket_addrs()
        .map_err(|source| Error::Io {
            path: host.into(),
            source,
        })?
        .next()
        .map(|addr| addr.ip())
        .ok_or_else(|| Error::Connect(format!("could not resolve {host}")))
}

fn reply_reason(code: u8) -> &'static str {
    match code {
        0x01 => "general SOCKS server failure",
        0x02 => "connection not allowed by ruleset",
        0x03 => "network unreachable",
        0x04 => "host unreachable",
        0x05 => "connection refused",
        0x06 => "TTL expired",
        0x07 => "command not supported",
        0x08 => "address type not supported",
        _ => "unknown error",
    }
}

fn write_all(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    stream.write_all(data).map_err(|source| Error::Io {
        path: "<socks5 proxy>".into(),
        source,
    })
}

fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> Result<()> {
    stream.read_exact(buf).map_err(|source| Error::Io {
        path: "<socks5 proxy>".into(),
        source,
    })
}

fn discard(stream: &mut TcpStream, len: usize) -> Result<()> {
    let mut buf = vec![0u8; len];
    read_exact(stream, &mut buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn connects_with_proxy_side_resolution_and_no_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            let mut greeting = [0u8; 3];
            stream.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting, [0x05, 0x01, 0x00]);
            stream.write_all(&[0x05, 0x00]).unwrap();

            let mut header = [0u8; 4];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(header, [0x05, 0x01, 0x00, 0x03]);

            let mut len = [0u8; 1];
            stream.read_exact(&mut len).unwrap();
            let mut domain = vec![0u8; len[0] as usize];
            stream.read_exact(&mut domain).unwrap();
            assert_eq!(domain, b"target.example");
            let mut port = [0u8; 2];
            stream.read_exact(&mut port).unwrap();
            assert_eq!(u16::from_be_bytes(port), 443);

            stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .unwrap();
        });

        let result = connect(
            "127.0.0.1",
            addr.port(),
            None,
            "target.example",
            443,
            ResolveMode::Proxy,
            None,
        );
        server.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn connects_with_client_side_resolution_for_an_ip_literal() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting = [0u8; 3];
            stream.read_exact(&mut greeting).unwrap();
            stream.write_all(&[0x05, 0x00]).unwrap();

            let mut header = [0u8; 4];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(header, [0x05, 0x01, 0x00, 0x01]);
            let mut rest = [0u8; 6];
            stream.read_exact(&mut rest).unwrap();
            assert_eq!(&rest[0..4], &[93, 184, 216, 34]);

            stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .unwrap();
        });

        let result = connect(
            "127.0.0.1",
            addr.port(),
            None,
            "93.184.216.34",
            80,
            ResolveMode::Client,
            None,
        );
        server.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn negotiates_username_password_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            let mut greeting = [0u8; 4];
            stream.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting, [0x05, 0x02, 0x00, 0x02]);
            stream.write_all(&[0x05, 0x02]).unwrap();

            let mut header = [0u8; 2];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(header[0], 0x01);
            let mut user = vec![0u8; header[1] as usize];
            stream.read_exact(&mut user).unwrap();
            assert_eq!(user, b"alice");
            let mut plen = [0u8; 1];
            stream.read_exact(&mut plen).unwrap();
            let mut pass = vec![0u8; plen[0] as usize];
            stream.read_exact(&mut pass).unwrap();
            assert_eq!(pass, b"s3cr3t");
            stream.write_all(&[0x01, 0x00]).unwrap();

            let mut rest = [0u8; 4 + 1 + 14 + 2];
            stream.read_exact(&mut rest).unwrap();
            stream
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .unwrap();
        });

        let credentials = ("alice".to_string(), Some("s3cr3t".to_string()));
        let result = connect(
            "127.0.0.1",
            addr.port(),
            Some(&credentials),
            "target.example",
            443,
            ResolveMode::Proxy,
            None,
        );
        server.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn surfaces_a_connect_failure_reply_as_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting = [0u8; 3];
            stream.read_exact(&mut greeting).unwrap();
            stream.write_all(&[0x05, 0x00]).unwrap();

            let mut header = [0u8; 4];
            stream.read_exact(&mut header).unwrap();
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).unwrap();
            let mut rest = vec![0u8; len[0] as usize + 2];
            stream.read_exact(&mut rest).unwrap();

            // 0x05 = connection refused.
            stream
                .write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .unwrap();
        });

        let result = connect(
            "127.0.0.1",
            addr.port(),
            None,
            "target.example",
            443,
            ResolveMode::Proxy,
            None,
        );
        server.join().unwrap();
        assert!(result.is_err());
    }
}
