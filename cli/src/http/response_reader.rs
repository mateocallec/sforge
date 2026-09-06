use std::io::{BufRead, BufReader, Read};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct RawResponse {
    pub status_code: u16,
    pub reason: String,
    /// As received: casing and order preserved, duplicates kept —
    /// symmetric with the byte-exact philosophy on the request side.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub fn read_response<R: Read>(reader: R, request_method: &str) -> Result<RawResponse> {
    let mut reader = BufReader::new(reader);

    let status_line = read_line(&mut reader)?;
    let (status_code, reason) = parse_status_line(&status_line)?;

    let mut headers = Vec::new();
    loop {
        let line = read_line(&mut reader)?;
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| Error::MalformedResponse(format!("invalid header line: {line}")))?;
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }

    let has_no_body = request_method.eq_ignore_ascii_case("HEAD")
        || matches!(status_code, 204 | 304)
        || (100..200).contains(&status_code);

    let body = if has_no_body {
        Vec::new()
    } else {
        read_body(&mut reader, &headers)?
    };

    Ok(RawResponse {
        status_code,
        reason,
        headers,
        body,
    })
}

fn parse_status_line(line: &str) -> Result<(u16, String)> {
    let mut parts = line.splitn(3, ' ');
    let http_version = parts
        .next()
        .ok_or_else(|| Error::MalformedResponse(format!("empty status line: {line}")))?;
    if !http_version.starts_with("HTTP/") {
        return Err(Error::MalformedResponse(format!(
            "not an HTTP response: {line}"
        )));
    }
    let status_code: u16 = parts
        .next()
        .ok_or_else(|| Error::MalformedResponse(format!("missing status code: {line}")))?
        .parse()
        .map_err(|_| Error::MalformedResponse(format!("invalid status code: {line}")))?;
    let reason = parts.next().unwrap_or("").to_string();
    Ok((status_code, reason))
}

fn read_body<R: BufRead>(reader: &mut R, headers: &[(String, String)]) -> Result<Vec<u8>> {
    if let Some(te) = header_value(headers, "transfer-encoding")
        && te.to_ascii_lowercase().contains("chunked")
    {
        return super::chunked::decode(reader);
    }

    if let Some(cl) = header_value(headers, "content-length") {
        let len: usize = cl
            .trim()
            .parse()
            .map_err(|_| Error::MalformedResponse(format!("invalid Content-Length: {cl}")))?;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).map_err(|source| Error::Io {
            path: "<socket>".into(),
            source,
        })?;
        return Ok(buf);
    }

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).map_err(|source| Error::Io {
        path: "<socket>".into(),
        source,
    })?;
    Ok(buf)
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Reads one CRLF- or LF-terminated line, minus the terminator. Shared with
/// `chunked.rs`, which parses chunk-size lines with the same grammar.
pub(super) fn read_line<R: BufRead>(reader: &mut R) -> Result<String> {
    let mut bytes = Vec::new();
    let n = reader
        .read_until(b'\n', &mut bytes)
        .map_err(|source| Error::Io {
            path: "<socket>".into(),
            source,
        })?;
    if n == 0 {
        return Err(Error::MalformedResponse(
            "connection closed before a complete response was received".to_string(),
        ));
    }
    while bytes.last() == Some(&b'\n') || bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_status_line_and_headers() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";
        let response = read_response(Cursor::new(raw.to_vec()), "GET").unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.reason, "OK");
        assert_eq!(response.header("content-type"), Some("text/plain"));
        assert_eq!(response.body, b"hello");
    }

    #[test]
    fn preserves_header_casing_and_duplicates() {
        let raw =
            b"HTTP/1.1 200 OK\r\nX-Weird-CASE: 1\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\n\r\n";
        let response = read_response(Cursor::new(raw.to_vec()), "GET").unwrap();
        assert!(response.headers.iter().any(|(n, _)| n == "X-Weird-CASE"));
        let cookies: Vec<&str> = response
            .headers
            .iter()
            .filter(|(n, _)| n == "Set-Cookie")
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(cookies, vec!["a=1", "b=2"]);
    }

    #[test]
    fn head_response_has_no_body_even_with_content_length() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        let response = read_response(Cursor::new(raw.to_vec()), "HEAD").unwrap();
        assert_eq!(response.body, Vec::<u8>::new());
    }

    #[test]
    fn no_content_length_reads_until_close() {
        let raw = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nwhatever remains";
        let response = read_response(Cursor::new(raw.to_vec()), "GET").unwrap();
        assert_eq!(response.body, b"whatever remains");
    }

    #[test]
    fn chunked_response_is_decoded() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        let response = read_response(Cursor::new(raw.to_vec()), "GET").unwrap();
        assert_eq!(response.body, b"hello");
    }

    #[test]
    fn conflicting_chunked_and_content_length_prefers_chunked() {
        // A real smuggling signal (RFC 7230 §3.3.3): when both are
        // present, Transfer-Encoding wins and Content-Length is ignored.
        let raw =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 999\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        let response = read_response(Cursor::new(raw.to_vec()), "GET").unwrap();
        assert_eq!(response.body, b"hello");
    }

    #[test]
    fn rejects_non_http_status_line() {
        let raw = b"not an http response\r\n\r\n";
        assert!(read_response(Cursor::new(raw.to_vec()), "GET").is_err());
    }
}
