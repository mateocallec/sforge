use std::path::PathBuf;

use crate::error::{Error, Result};

pub enum FormPart {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        path: PathBuf,
        filename: Option<String>,
        content_type: Option<String>,
    },
}

pub struct MultipartBody {
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Parses one `-F`/`--form` value using curl's own grammar:
/// `name=value`, `name=@path[;type=...][;filename=...]` (upload a file),
/// or `name=<path` (use the given file's contents as the field's text
/// value, not as an upload).
pub fn parse_form_field(raw: &str) -> Result<FormPart> {
    let (name, rest) = raw
        .split_once('=')
        .ok_or_else(|| Error::InvalidForm(raw.to_string()))?;
    if name.is_empty() {
        return Err(Error::InvalidForm(raw.to_string()));
    }

    if let Some(spec) = rest.strip_prefix('@') {
        let mut segments = spec.split(';');
        let path = segments.next().unwrap_or("");
        if path.is_empty() {
            return Err(Error::InvalidForm(raw.to_string()));
        }

        let mut filename = None;
        let mut content_type = None;
        for segment in segments {
            if let Some(v) = segment.strip_prefix("filename=") {
                filename = Some(v.to_string());
            } else if let Some(v) = segment.strip_prefix("type=") {
                content_type = Some(v.to_string());
            }
        }

        return Ok(FormPart::File {
            name: name.to_string(),
            path: PathBuf::from(path),
            filename,
            content_type,
        });
    }

    if let Some(path) = rest.strip_prefix('<') {
        let value = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
        return Ok(FormPart::Text {
            name: name.to_string(),
            value,
        });
    }

    Ok(FormPart::Text {
        name: name.to_string(),
        value: rest.to_string(),
    })
}

pub fn build(parts: &[FormPart]) -> Result<MultipartBody> {
    let boundary = format!("sforge-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();

    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match part {
            FormPart::Text { name, value } => {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                );
                body.extend_from_slice(value.as_bytes());
            }
            FormPart::File {
                name,
                path,
                filename,
                content_type,
            } => {
                let filename = filename.clone().unwrap_or_else(|| {
                    path.file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default()
                });
                let content_type = content_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )
                    .as_bytes(),
                );
                let contents = std::fs::read(path).map_err(|source| Error::Io {
                    path: path.clone(),
                    source,
                })?;
                body.extend_from_slice(&contents);
            }
        }
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    Ok(MultipartBody {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_field() {
        match parse_form_field("name=value").unwrap() {
            FormPart::Text { name, value } => {
                assert_eq!(name, "name");
                assert_eq!(value, "value");
            }
            _ => panic!("expected a text field"),
        }
    }

    #[test]
    fn parses_file_upload_with_filename_and_type() {
        match parse_form_field("avatar=@/tmp/pic.png;type=image/png;filename=me.png").unwrap() {
            FormPart::File {
                name,
                path,
                filename,
                content_type,
            } => {
                assert_eq!(name, "avatar");
                assert_eq!(path, PathBuf::from("/tmp/pic.png"));
                assert_eq!(filename.as_deref(), Some("me.png"));
                assert_eq!(content_type.as_deref(), Some("image/png"));
            }
            _ => panic!("expected a file field"),
        }
    }

    #[test]
    fn rejects_field_without_equals() {
        assert!(parse_form_field("nope").is_err());
    }

    #[test]
    fn builds_multipart_body_with_boundary_delimiters() {
        let parts = vec![FormPart::Text {
            name: "a".to_string(),
            value: "1".to_string(),
        }];
        let built = build(&parts).unwrap();
        assert!(
            built
                .content_type
                .starts_with("multipart/form-data; boundary=sforge-")
        );
        let text = String::from_utf8(built.body).unwrap();
        assert!(text.contains("Content-Disposition: form-data; name=\"a\""));
        assert!(text.contains("\r\n1\r\n"));
        assert!(text.trim_end().ends_with("--"));
    }
}
