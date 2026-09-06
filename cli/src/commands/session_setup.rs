use std::io::Write;

use crate::args::session_args::SessionSetupArgs;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::session::cookie::CookieEntry;
use crate::session::{Auth, AuthKind, HeaderEntry, SessionPatch, Store};

pub fn run(config: &Config, args: SessionSetupArgs) -> Result<()> {
    let store = Store::new(config.sessions_dir.clone());

    if args.has_inline_data() {
        let patch = build_patch(&args)?;
        let session = store.replace(&args.name, patch)?;
        println!(
            "Session \"{}\" saved ({} cookie(s), {} header(s)).",
            session.meta.name,
            session.cookies.len(),
            session.headers.len()
        );
        return Ok(());
    }

    edit_in_place(&store, &args.name)
}

/// The no-inline-data path: open the session file in $EDITOR, re-validate
/// on close, and offer to reopen rather than silently discarding or
/// "fixing" whatever the user wrote.
fn edit_in_place(store: &Store, name: &str) -> Result<()> {
    let (id, path) = store.ensure_editable(name)?;

    loop {
        crate::editor::open(&path)?;

        match store.load(id) {
            Ok(_) => {
                store.touch_updated_at(id)?;
                println!("Session \"{name}\" saved.");
                return Ok(());
            }
            Err(err) => {
                eprintln!("sforge: {err}");
                if !prompt_reopen()? {
                    eprintln!("Left unchanged at {}", path.display());
                    return Ok(());
                }
            }
        }
    }
}

fn prompt_reopen() -> Result<bool> {
    eprint!("Reopen in the editor to fix? [Y/n] ");
    std::io::stderr().flush().ok();

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|source| Error::Io {
            path: "<stdin>".into(),
            source,
        })?;

    let answer = input.trim().to_ascii_lowercase();
    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

fn build_patch(args: &SessionSetupArgs) -> Result<SessionPatch> {
    let mut patch = SessionPatch::default();

    if !args.set_cookie.is_empty() {
        let mut cookies = Vec::with_capacity(args.set_cookie.len());
        for raw in &args.set_cookie {
            cookies.push(CookieEntry::parse_set_cookie(raw, None)?);
        }
        patch.cookies = Some(cookies);
    }

    if !args.header.is_empty() {
        let mut headers = Vec::with_capacity(args.header.len());
        for raw in &args.header {
            headers.push(parse_header(raw)?);
        }
        patch.headers = Some(headers);
    }

    if let Some(token) = &args.bearer {
        patch.auth = Some(Auth {
            kind: Some(AuthKind::Bearer),
            username: None,
            password: Some(token.clone()),
        });
    } else if let Some(user_pass) = &args.user {
        let (username, password) = match user_pass.split_once(':') {
            Some((u, p)) => (u.to_string(), Some(p.to_string())),
            None => (user_pass.clone(), None),
        };
        let kind = if args.digest {
            AuthKind::Digest
        } else {
            AuthKind::Basic
        };
        patch.auth = Some(Auth {
            kind: Some(kind),
            username: Some(username),
            password,
        });
    }

    Ok(patch)
}

fn parse_header(raw: &str) -> Result<HeaderEntry> {
    let (name, value) = raw
        .split_once(':')
        .ok_or_else(|| Error::InvalidHeader(raw.to_string()))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::InvalidHeader(raw.to_string()));
    }
    Ok(HeaderEntry {
        name,
        value: value.trim().to_string(),
    })
}
