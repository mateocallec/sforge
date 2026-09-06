use std::io::Read;

use crate::error::{Error, Result};

/// Display-only decompression, gated by the caller on whether
/// `--compressed` was passed — never applied to what gets sent, only to
/// how a response is interpreted for output. Without `--compressed`, raw
/// (possibly still-encoded) bytes pass straight through untouched, so a
/// Content-Encoding mismatch stays inspectable.
pub fn decompress(body: &[u8], content_encoding: Option<&str>) -> Result<Vec<u8>> {
    let Some(encoding) = content_encoding else {
        return Ok(body.to_vec());
    };

    match encoding.trim().to_ascii_lowercase().as_str() {
        "gzip" | "x-gzip" => decode_gzip(body),
        "deflate" => decode_deflate(body),
        "br" => decode_brotli(body),
        "identity" | "" => Ok(body.to_vec()),
        other => Err(Error::UnsupportedContentEncoding(other.to_string())),
    }
}

fn decode_gzip(body: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(body);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|source| Error::Io {
        path: "<response body>".into(),
        source,
    })?;
    Ok(out)
}

fn decode_deflate(body: &[u8]) -> Result<Vec<u8>> {
    let mut zlib = flate2::read::ZlibDecoder::new(body);
    let mut out = Vec::new();
    if zlib.read_to_end(&mut out).is_ok() {
        return Ok(out);
    }

    // Some servers send raw DEFLATE (no zlib wrapper) despite labeling it
    // "deflate" — a long-standing real-world interop quirk; fall back.
    out.clear();
    let mut raw = flate2::read::DeflateDecoder::new(body);
    raw.read_to_end(&mut out).map_err(|source| Error::Io {
        path: "<response body>".into(),
        source,
    })?;
    Ok(out)
}

fn decode_brotli(body: &[u8]) -> Result<Vec<u8>> {
    let mut input = body;
    let mut out = Vec::new();
    brotli::BrotliDecompress(&mut input, &mut out).map_err(|e| Error::Decompress(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn passes_through_when_no_content_encoding() {
        assert_eq!(decompress(b"raw", None).unwrap(), b"raw");
    }

    #[test]
    fn passes_through_identity() {
        assert_eq!(decompress(b"raw", Some("identity")).unwrap(), b"raw");
    }

    #[test]
    fn decodes_gzip() {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"hello world").unwrap();
        let compressed = encoder.finish().unwrap();

        assert_eq!(
            decompress(&compressed, Some("gzip")).unwrap(),
            b"hello world"
        );
    }

    #[test]
    fn decodes_zlib_wrapped_deflate() {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"hello world").unwrap();
        let compressed = encoder.finish().unwrap();

        assert_eq!(
            decompress(&compressed, Some("deflate")).unwrap(),
            b"hello world"
        );
    }

    #[test]
    fn rejects_unsupported_encoding() {
        let result = decompress(b"raw", Some("zstd"));
        assert!(matches!(result, Err(Error::UnsupportedContentEncoding(_))));
    }
}
