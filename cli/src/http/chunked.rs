use std::io::BufRead;

use super::response_reader::read_line;
use crate::error::{Error, Result};

pub fn decode<R: BufRead>(reader: &mut R) -> Result<Vec<u8>> {
    let mut body = Vec::new();

    loop {
        let size_line = read_line(reader)?;
        let size_str = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_str, 16)
            .map_err(|_| Error::MalformedResponse(format!("invalid chunk size: {size_line}")))?;

        if size == 0 {
            // Optional trailer headers, terminated by an empty line;
            // discarded rather than merged into the response's own headers.
            loop {
                let trailer = read_line(reader)?;
                if trailer.is_empty() {
                    break;
                }
            }
            break;
        }

        let mut chunk = vec![0u8; size];
        reader.read_exact(&mut chunk).map_err(|source| Error::Io {
            path: "<socket>".into(),
            source,
        })?;
        body.extend_from_slice(&chunk);

        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf).map_err(|source| Error::Io {
            path: "<socket>".into(),
            source,
        })?;
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn decodes_multiple_chunks() {
        let raw = b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        let mut reader = Cursor::new(raw.to_vec());
        assert_eq!(decode(&mut reader).unwrap(), b"Wikipedia");
    }

    #[test]
    fn decodes_empty_body() {
        let raw = b"0\r\n\r\n";
        let mut reader = Cursor::new(raw.to_vec());
        assert_eq!(decode(&mut reader).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn discards_trailer_headers() {
        let raw = b"3\r\nfoo\r\n0\r\nX-Trailer: value\r\n\r\n";
        let mut reader = Cursor::new(raw.to_vec());
        assert_eq!(decode(&mut reader).unwrap(), b"foo");
    }

    #[test]
    fn chunk_extensions_are_ignored() {
        let raw = b"3;ext=1\r\nfoo\r\n0\r\n\r\n";
        let mut reader = Cursor::new(raw.to_vec());
        assert_eq!(decode(&mut reader).unwrap(), b"foo");
    }
}
