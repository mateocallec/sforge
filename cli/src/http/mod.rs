pub mod chunked;
pub mod connection;
pub mod cookie_jar;
pub mod decompress;
pub mod multipart;
pub mod proxy;
pub mod redirect;
pub mod request_writer;
pub mod response_reader;
pub mod retry;
pub mod socks5;
pub mod tls;
pub mod url;

use std::io::Write;
use std::time::Duration;

use connection::Connection;
use request_writer::RawRequest;
use response_reader::RawResponse;
use tls::TlsOptions;
use url::{RequestUrl, Scheme};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub enum ProxyConfig {
    Http {
        host: String,
        port: u16,
        userinfo: Option<(String, Option<String>)>,
    },
    Socks5 {
        host: String,
        port: u16,
        userinfo: Option<(String, Option<String>)>,
        resolve_mode: socks5::ResolveMode,
    },
}

#[derive(Default)]
pub struct SendOptions {
    pub connect_timeout: Option<Duration>,
    pub tls: TlsOptions,
    pub proxy: Option<ProxyConfig>,
    /// `--resolve host:port:address` overrides: connect to `address`
    /// instead of resolving `host`, while the `Host` header and TLS SNI
    /// still carry the original `host` — vhost testing and DNS-based
    /// access-control bypass, core pentest functionality.
    pub resolve_overrides: Vec<(String, u16, String)>,
}

impl SendOptions {
    fn resolved_host<'a>(&'a self, url: &'a RequestUrl) -> &'a str {
        self.resolve_overrides
            .iter()
            .find(|(host, port, _)| host == &url.host && *port == url.port)
            .map(|(_, _, address)| address.as_str())
            .unwrap_or(&url.host)
    }
}

/// The outcome of [`establish`]: a connection, the request target to put
/// on the request line (rewritten to absolute-form when relayed through a
/// plain-HTTP proxy), and an extra header to attach to the request when
/// relevant (`Proxy-Authorization` for that same plain-HTTP-relay case —
/// the `CONNECT` tunnel case authenticates as part of establishing the
/// tunnel itself and needs nothing added here).
pub struct Established {
    pub connection: Connection,
    pub target: String,
    pub extra_header: Option<(String, String)>,
}

/// Establishes a connection for `url` under `opts`: direct, through an
/// HTTP proxy, or through SOCKS5.
pub fn establish(url: &RequestUrl, opts: &SendOptions) -> Result<Established> {
    match &opts.proxy {
        None => {
            let connect_host = opts.resolved_host(url);
            match url.scheme {
                Scheme::Http => {
                    let tcp =
                        Connection::connect_plain(connect_host, url.port, opts.connect_timeout)?;
                    Ok(Established {
                        connection: Connection::Plain(tcp),
                        target: url.raw_target.clone(),
                        extra_header: None,
                    })
                }
                Scheme::Https => {
                    let tls_config = tls::build_config(&opts.tls)?;
                    let conn = Connection::connect_tls(
                        connect_host,
                        url.port,
                        &url.host,
                        tls_config,
                        opts.connect_timeout,
                    )?;
                    Ok(Established {
                        connection: conn,
                        target: url.raw_target.clone(),
                        extra_header: None,
                    })
                }
            }
        }
        Some(ProxyConfig::Http {
            host,
            port,
            userinfo,
        }) => {
            let target = proxy::ProxyTarget {
                host: host.clone(),
                port: *port,
                userinfo: userinfo.clone(),
            };
            match url.scheme {
                Scheme::Http => {
                    let tcp = Connection::connect_plain(host, *port, opts.connect_timeout)?;
                    let absolute =
                        format!("http://{}{}", url.default_host_header(), url.raw_target);
                    Ok(Established {
                        connection: Connection::Plain(tcp),
                        target: absolute,
                        extra_header: proxy::proxy_authorization(&target),
                    })
                }
                Scheme::Https => {
                    let tcp =
                        proxy::connect_tunnel(&target, &url.host, url.port, opts.connect_timeout)?;
                    let tls_config = tls::build_config(&opts.tls)?;
                    let conn = Connection::wrap_tls(tcp, &url.host, tls_config)?;
                    Ok(Established {
                        connection: conn,
                        target: url.raw_target.clone(),
                        extra_header: None,
                    })
                }
            }
        }
        Some(ProxyConfig::Socks5 {
            host,
            port,
            userinfo,
            resolve_mode,
        }) => {
            let tcp = socks5::connect(
                host,
                *port,
                userinfo.as_ref(),
                &url.host,
                url.port,
                *resolve_mode,
                opts.connect_timeout,
            )?;
            match url.scheme {
                Scheme::Http => Ok(Established {
                    connection: Connection::Plain(tcp),
                    target: url.raw_target.clone(),
                    extra_header: None,
                }),
                Scheme::Https => {
                    let tls_config = tls::build_config(&opts.tls)?;
                    let conn = Connection::wrap_tls(tcp, &url.host, tls_config)?;
                    Ok(Established {
                        connection: conn,
                        target: url.raw_target.clone(),
                        extra_header: None,
                    })
                }
            }
        }
    }
}

/// Writes `raw` to an already-established connection and reads back the
/// response. Splitting this from [`establish`] is what lets a caller
/// driving a redirect loop reconnect per hop while keeping the connection
/// step and the send/receive step independently testable.
pub fn send(mut conn: Connection, raw: &RawRequest) -> Result<RawResponse> {
    let bytes = raw.to_bytes();
    conn.write_all(&bytes).map_err(|source| Error::Io {
        path: "<socket>".into(),
        source,
    })?;
    conn.flush().map_err(|source| Error::Io {
        path: "<socket>".into(),
        source,
    })?;
    response_reader::read_response(conn, &raw.method)
}

/// Convenience for the common case: connect directly (or through
/// `opts.proxy`) and send in one call, discarding the resolved target
/// string since the caller already built `raw.target` to match.
pub fn send_once(raw: &RawRequest, url: &RequestUrl, opts: &SendOptions) -> Result<RawResponse> {
    let established = establish(url, opts)?;
    send(established.connection, raw)
}
