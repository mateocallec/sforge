use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConnection, StreamOwned};

use crate::error::{Error, Result};

/// A connected socket, plain or TLS-wrapped behind one `Read + Write`
/// surface so the rest of the engine doesn't need to know which.
pub enum Connection {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Connection {
    pub fn connect_plain(host: &str, port: u16, timeout: Option<Duration>) -> Result<TcpStream> {
        let addr = resolve(host, port)?;
        let stream = match timeout {
            Some(timeout) => TcpStream::connect_timeout(&addr, timeout),
            None => TcpStream::connect(addr),
        }
        .map_err(|source| Error::Io {
            path: format!("{host}:{port}").into(),
            source,
        })?;
        stream.set_nodelay(true).ok();
        Ok(stream)
    }

    /// `host`/`port` are what we TCP-connect to; `sni_name` is what goes
    /// into the TLS handshake's SNI extension — kept separate so
    /// `--resolve`/`--connect-to` can redirect the connection while the
    /// Host header and SNI still carry the original hostname.
    pub fn connect_tls(
        host: &str,
        port: u16,
        sni_name: &str,
        tls_config: Arc<rustls::ClientConfig>,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        let tcp = Self::connect_plain(host, port, timeout)?;
        Self::wrap_tls(tcp, sni_name, tls_config)
    }

    /// Runs a TLS handshake over an already-connected socket — the shared
    /// tail end of both a direct HTTPS connection and an HTTPS-through-a-
    /// proxy `CONNECT` tunnel or SOCKS5 relay.
    pub fn wrap_tls(
        tcp: TcpStream,
        sni_name: &str,
        tls_config: Arc<rustls::ClientConfig>,
    ) -> Result<Self> {
        let bare_sni = sni_name.trim_start_matches('[').trim_end_matches(']');
        let server_name = ServerName::try_from(bare_sni.to_string())
            .map_err(|_| Error::InvalidUrl(sni_name.to_string()))?;
        let conn = ClientConnection::new(tls_config, server_name)
            .map_err(|e| Error::Tls(e.to_string()))?;
        Ok(Connection::Tls(Box::new(StreamOwned::new(conn, tcp))))
    }
}

fn resolve(host: &str, port: u16) -> Result<SocketAddr> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = IpAddr::from_str(bare) {
        return Ok(SocketAddr::new(ip, port));
    }
    (host, port)
        .to_socket_addrs()
        .map_err(|source| Error::Io {
            path: format!("{host}:{port}").into(),
            source,
        })?
        .next()
        .ok_or_else(|| Error::Connect(format!("could not resolve {host}")))
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Connection::Plain(s) => s.read(buf),
            Connection::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Connection::Plain(s) => s.write(buf),
            Connection::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Connection::Plain(s) => s.flush(),
            Connection::Tls(s) => s.flush(),
        }
    }
}
