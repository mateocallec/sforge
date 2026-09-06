use std::io::Write as _;
use std::time::Duration;

use crate::args::request_args::RequestArgs;
use crate::auth::digest::DigestChallenge;
use crate::auth::{self, AuthPlan};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::request_writer::{HeaderDirective, RawRequest, RequestSpec, build};
use crate::http::response_reader::RawResponse;
use crate::http::url::RequestUrl;
use crate::http::{
    ProxyConfig, SendOptions, cookie_jar, decompress, establish, multipart, redirect, retry, send,
    socks5,
};
use crate::output::color::{self, ColorMode};
use crate::output::{headers_view, json_highlight, trace};
use crate::session::cookie::CookieEntry;
use crate::session::{Auth, AuthKind, Store};

pub fn run(config: &Config, args: RequestArgs) -> Result<()> {
    let url_str = args
        .url
        .clone()
        .ok_or_else(|| Error::InvalidUrl("no URL given".to_string()))?;
    let url = RequestUrl::parse(&url_str)?;

    let session = match &args.session {
        Some(name) => Some(Store::new(config.sessions_dir.clone()).load_by_name(name)?),
        None => None,
    };

    let method = resolve_method(&args, session.is_some());
    let base_directives = build_base_directives(&args, session.as_ref())?;
    let (body, content_type_default) = build_body(&args)?;
    let auth = resolve_auth(&args, session.as_ref());
    let cookies = build_cookie_set(&args, session.as_ref())?;
    let manual_cookies = args.cookie.clone().unwrap_or_default();

    let user_agent = args
        .user_agent
        .clone()
        .unwrap_or_else(|| format!("sforge/{}", env!("CARGO_PKG_VERSION")));

    let send_opts = SendOptions {
        connect_timeout: args.connect_timeout.map(Duration::from_secs_f64),
        tls: crate::http::tls::TlsOptions {
            insecure: args.insecure,
            ca_cert_path: args.cacert.clone(),
            client_cert_path: args.cert.clone(),
            client_key_path: args.key.clone(),
        },
        proxy: resolve_proxy(&args)?,
        resolve_overrides: parse_resolve_overrides(&args.resolve)?,
    };

    let retry_policy = crate::http::retry::RetryPolicy {
        max_retries: args.retry,
        delay: Duration::from_secs(args.retry_delay),
    };

    let mut current_url = url;
    let mut current_method = method;
    let mut current_body = body;
    let mut redirect_count = 0u32;
    let cnonce = auth::fresh_cnonce();

    let (final_request, final_response) = loop {
        let target_method = if args.head {
            "HEAD".to_string()
        } else {
            current_method.clone()
        };

        let cookie_header = combined_cookie_header(&cookies, &manual_cookies, &current_url);
        let auth_directive = match auth::plan(&auth) {
            AuthPlan::Immediate(d) => Some(d),
            _ => None,
        };

        let raw_request = build_request(
            &target_method,
            &current_url,
            &base_directives,
            auth_directive,
            &user_agent,
            args.compressed,
            args.chunked,
            cookie_header.clone(),
            content_type_default.clone(),
            &current_body,
        );

        let response = retry::with_retry(&retry_policy, || {
            send_via(&raw_request, &current_url, &send_opts)
        })?;

        let (raw_request, response) = maybe_retry_with_digest(
            &raw_request,
            &response,
            &auth,
            &target_method,
            &current_url,
            &base_directives,
            &user_agent,
            args.compressed,
            args.chunked,
            cookie_header,
            content_type_default.clone(),
            &current_body,
            &cnonce,
            &send_opts,
        )?;

        if args.verbose {
            eprint!("{}", trace::render_sent(&raw_request.to_bytes()));
            eprint!(
                "{}",
                trace::render_received_headers(
                    &format!("HTTP/1.1 {} {}", response.status_code, response.reason),
                    &response.headers
                )
            );
        }

        if !args.location {
            break (raw_request, response);
        }
        let Some(policy) = redirect::body_policy_for(response.status_code) else {
            break (raw_request, response);
        };
        let Some(location) = response.header("location") else {
            break (raw_request, response);
        };
        let Some(next_url) = redirect::resolve_location(&current_url, location) else {
            break (raw_request, response);
        };

        redirect_count += 1;
        if redirect_count > args.max_redirs {
            return Err(Error::TooManyRedirects(args.max_redirs));
        }

        if policy == redirect::BodyPolicy::SwitchToGet {
            current_method = "GET".to_string();
            current_body = Vec::new();
        }
        current_url = next_url;
    };

    render_output(&args, &final_request, &final_response)
}

fn resolve_method(args: &RequestArgs, has_session_or_body: bool) -> String {
    if let Some(m) = &args.method {
        return m.to_ascii_uppercase();
    }
    let has_body = !args.data.is_empty()
        || !args.data_raw.is_empty()
        || args.data_binary.is_some()
        || !args.data_urlencode.is_empty()
        || args.json.is_some()
        || !args.form.is_empty()
        || args.upload_file.is_some();
    if has_body && !args.get {
        "POST".to_string()
    } else {
        let _ = has_session_or_body;
        "GET".to_string()
    }
}

fn build_base_directives(
    args: &RequestArgs,
    session: Option<&crate::session::Session>,
) -> Result<Vec<HeaderDirective>> {
    let mut directives = Vec::new();
    if let Some(session) = session {
        for h in &session.headers {
            directives.push(HeaderDirective::Set(h.name.clone(), h.value.clone()));
        }
    }
    if let Some(referer) = &args.referer {
        directives.push(HeaderDirective::Set("Referer".to_string(), referer.clone()));
    }
    for raw in &args.header {
        directives.push(parse_header_directive(raw)?);
    }
    Ok(directives)
}

fn parse_header_directive(raw: &str) -> Result<HeaderDirective> {
    let (name, rest) = raw
        .split_once(':')
        .ok_or_else(|| Error::InvalidHeader(raw.to_string()))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::InvalidHeader(raw.to_string()));
    }
    if rest.is_empty() {
        return Ok(HeaderDirective::Suppress(name));
    }
    let value = rest.strip_prefix(';').unwrap_or(rest).trim();
    Ok(HeaderDirective::Set(name, value.to_string()))
}

fn build_body(args: &RequestArgs) -> Result<(Vec<u8>, Option<String>)> {
    if let Some(json) = &args.json {
        let value = read_at_value(json)?;
        return Ok((value.into_bytes(), Some("application/json".to_string())));
    }

    if !args.form.is_empty() {
        let parts: Vec<multipart::FormPart> = args
            .form
            .iter()
            .map(|f| multipart::parse_form_field(f))
            .collect::<Result<_>>()?;
        let built = multipart::build(&parts)?;
        return Ok((built.body, Some(built.content_type)));
    }

    if let Some(path) = &args.upload_file {
        let contents = std::fs::read(path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        return Ok((contents, None));
    }

    if let Some(raw) = &args.data_binary {
        let value = read_at_value(raw)?;
        return Ok((
            value.into_bytes(),
            Some("application/x-www-form-urlencoded".to_string()),
        ));
    }

    let mut pieces = Vec::new();
    for raw in &args.data {
        let value = read_at_value(raw)?;
        pieces.push(strip_cr_lf(&value));
    }
    for raw in &args.data_raw {
        pieces.push(raw.clone());
    }
    for raw in &args.data_urlencode {
        pieces.push(url_encode_field(raw)?);
    }

    if pieces.is_empty() {
        return Ok((Vec::new(), None));
    }

    Ok((
        pieces.join("&").into_bytes(),
        Some("application/x-www-form-urlencoded".to_string()),
    ))
}

fn read_at_value(raw: &str) -> Result<String> {
    if let Some(path) = raw.strip_prefix('@') {
        if path == "-" {
            let mut buf = String::new();
            std::io::stdin()
                .read_line(&mut buf)
                .map_err(|source| Error::Io {
                    path: "<stdin>".into(),
                    source,
                })?;
            return Ok(buf);
        }
        return std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.into(),
            source,
        });
    }
    Ok(raw.to_string())
}

fn strip_cr_lf(s: &str) -> String {
    s.chars().filter(|c| *c != '\r' && *c != '\n').collect()
}

fn url_encode_field(raw: &str) -> Result<String> {
    if let Some(rest) = raw.strip_prefix('@') {
        let contents = std::fs::read_to_string(rest).map_err(|source| Error::Io {
            path: rest.into(),
            source,
        })?;
        return Ok(percent_encode(&contents));
    }
    if let Some((name, rest)) = raw.split_once('=') {
        if let Some(path) = rest.strip_prefix('@') {
            let contents = std::fs::read_to_string(path).map_err(|source| Error::Io {
                path: path.into(),
                source,
            })?;
            return Ok(format!(
                "{}={}",
                percent_encode(name),
                percent_encode(&contents)
            ));
        }
        return Ok(format!("{}={}", percent_encode(name), percent_encode(rest)));
    }
    if let Some(rest) = raw.strip_prefix('=') {
        return Ok(percent_encode(rest));
    }
    Ok(percent_encode(raw))
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn resolve_auth(args: &RequestArgs, session: Option<&crate::session::Session>) -> Auth {
    if let Some(token) = &args.bearer {
        return Auth {
            kind: Some(AuthKind::Bearer),
            username: None,
            password: Some(token.clone()),
        };
    }
    if let Some(user_pass) = &args.user {
        let (username, password) = match user_pass.split_once(':') {
            Some((u, p)) => (u.to_string(), Some(p.to_string())),
            None => (user_pass.clone(), None),
        };
        let kind = if args.digest {
            AuthKind::Digest
        } else {
            AuthKind::Basic
        };
        return Auth {
            kind: Some(kind),
            username: Some(username),
            password,
        };
    }
    session.map(|s| s.auth.clone()).unwrap_or_default()
}

fn build_cookie_set(
    args: &RequestArgs,
    session: Option<&crate::session::Session>,
) -> Result<Vec<CookieEntry>> {
    let mut cookies = session.map(|s| s.cookies.clone()).unwrap_or_default();
    for raw in &args.set_cookie {
        let default_domain = args
            .url
            .as_deref()
            .and_then(|u| RequestUrl::parse(u).ok())
            .map(|u| u.host);
        cookies.push(CookieEntry::parse_set_cookie(
            raw,
            default_domain.as_deref(),
        )?);
    }
    Ok(cookies)
}

fn combined_cookie_header(
    cookies: &[CookieEntry],
    manual: &str,
    url: &RequestUrl,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(matched) = cookie_jar::header_value(cookies, url) {
        parts.push(matched);
    }
    if !manual.is_empty() {
        parts.push(manual.to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn parse_resolve_overrides(raw: &[String]) -> Result<Vec<(String, u16, String)>> {
    raw.iter()
        .map(|entry| {
            let mut parts = entry.splitn(3, ':');
            let host = parts.next().filter(|s| !s.is_empty());
            let port = parts.next().and_then(|p| p.parse::<u16>().ok());
            let address = parts.next().filter(|s| !s.is_empty());
            match (host, port, address) {
                (Some(host), Some(port), Some(address)) => {
                    Ok((host.to_string(), port, address.to_string()))
                }
                _ => Err(Error::InvalidUrl(format!(
                    "invalid --resolve value \"{entry}\""
                ))),
            }
        })
        .collect()
}

fn resolve_proxy(args: &RequestArgs) -> Result<Option<ProxyConfig>> {
    let Some(raw) = &args.proxy else {
        return Ok(None);
    };

    let (scheme, rest) = raw.split_once("://").unwrap_or(("http", raw.as_str()));
    let (userinfo_str, host_port) = match rest.rsplit_once('@') {
        Some((info, hp)) => (Some(info), hp),
        None => (None, rest),
    };
    let userinfo = userinfo_str.map(|info| match info.split_once(':') {
        Some((u, p)) => (u.to_string(), Some(p.to_string())),
        None => (info.to_string(), None),
    });
    let (host, port_str) = host_port
        .rsplit_once(':')
        .ok_or_else(|| Error::InvalidProxy(raw.clone()))?;
    let port: u16 = port_str
        .parse()
        .map_err(|_| Error::InvalidProxy(raw.clone()))?;

    match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => Ok(Some(ProxyConfig::Http {
            host: host.to_string(),
            port,
            userinfo,
        })),
        "socks5" => Ok(Some(ProxyConfig::Socks5 {
            host: host.to_string(),
            port,
            userinfo,
            resolve_mode: socks5::ResolveMode::Client,
        })),
        "socks5h" => Ok(Some(ProxyConfig::Socks5 {
            host: host.to_string(),
            port,
            userinfo,
            resolve_mode: socks5::ResolveMode::Proxy,
        })),
        _ => Err(Error::InvalidProxy(raw.clone())),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_request(
    method: &str,
    url: &RequestUrl,
    base_directives: &[HeaderDirective],
    extra_auth: Option<HeaderDirective>,
    user_agent: &str,
    compressed: bool,
    chunked: bool,
    cookie_default: Option<String>,
    content_type_default: Option<String>,
    body: &[u8],
) -> RawRequest {
    let mut directives = base_directives.to_vec();
    if let Some(d) = extra_auth {
        directives.push(d);
    }
    let spec = RequestSpec {
        method: method.to_string(),
        target: url.raw_target.clone(),
        directives,
        host_default: url.default_host_header(),
        user_agent_default: user_agent.to_string(),
        accept_default: "*/*".to_string(),
        accept_encoding_default: if compressed {
            Some("gzip, deflate, br".to_string())
        } else {
            None
        },
        connection_default: "close".to_string(),
        cookie_default,
        content_type_default,
        body: body.to_vec(),
        force_chunked: chunked,
    };
    build(spec)
}

fn send_via(raw: &RawRequest, url: &RequestUrl, opts: &SendOptions) -> Result<RawResponse> {
    let established = establish(url, opts)?;
    let mut request = raw.clone();
    if let Some((name, value)) = established.extra_header {
        request.headers.insert(0, (name, value));
    }
    if established.target != raw.target {
        request.target = established.target;
    }
    send(established.connection, &request)
}

#[allow(clippy::too_many_arguments)]
fn maybe_retry_with_digest(
    raw_request: &RawRequest,
    response: &RawResponse,
    auth: &Auth,
    method: &str,
    url: &RequestUrl,
    base_directives: &[HeaderDirective],
    user_agent: &str,
    compressed: bool,
    chunked: bool,
    cookie_default: Option<String>,
    content_type_default: Option<String>,
    body: &[u8],
    cnonce: &str,
    send_opts: &SendOptions,
) -> Result<(RawRequest, RawResponse)> {
    if response.status_code != 401 || !matches!(auth::plan(auth), AuthPlan::NeedsChallenge) {
        return Ok((raw_request.clone(), response.clone()));
    }
    let Some(www_authenticate) = response.header("www-authenticate") else {
        return Ok((raw_request.clone(), response.clone()));
    };
    let Some(challenge) = DigestChallenge::parse(www_authenticate) else {
        return Ok((raw_request.clone(), response.clone()));
    };

    let username = auth.username.clone().unwrap_or_default();
    let password = auth.password.clone().unwrap_or_default();
    let digest = crate::auth::digest::compute(
        &challenge,
        &username,
        &password,
        method,
        &url.raw_target,
        cnonce,
        1,
    );
    let auth_directive = HeaderDirective::Set("Authorization".to_string(), digest.header_value);

    let retried_request = build_request(
        method,
        url,
        base_directives,
        Some(auth_directive),
        user_agent,
        compressed,
        chunked,
        cookie_default,
        content_type_default,
        body,
    );
    let retried_response = send_via(&retried_request, url, send_opts)?;
    Ok((retried_request, retried_response))
}

fn render_output(args: &RequestArgs, request: &RawRequest, response: &RawResponse) -> Result<()> {
    if let Some(path) = &args.dump_header {
        let mut text = format!("HTTP/1.1 {} {}\r\n", response.status_code, response.reason);
        for (name, value) in &response.headers {
            text.push_str(&format!("{name}: {value}\r\n"));
        }
        std::fs::write(path, text).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
    }

    let color_mode = ColorMode::parse(&args.color).unwrap_or_default();
    let colorize = color::enabled(color_mode);

    if !args.silent && args.include {
        println!(
            "{}",
            headers_view::render_status_line(response.status_code, &response.reason, colorize)
        );
        print!(
            "{}",
            headers_view::render_headers(&response.headers, colorize)
        );
        println!();
    }

    let body = if request.method.eq_ignore_ascii_case("HEAD") || args.head {
        Vec::new()
    } else {
        let content_encoding = response.header("content-encoding").map(str::to_string);
        let encoding_for_decompress = if args.compressed {
            content_encoding.as_deref()
        } else {
            None
        };
        decompress::decompress(&response.body, encoding_for_decompress)?
    };

    if let Some(path) = &args.output {
        std::fs::write(path, &body).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        return Ok(());
    }
    if args.remote_name {
        let filename = args
            .url
            .as_deref()
            .and_then(|u| u.rsplit('/').next())
            .filter(|s| !s.is_empty())
            .unwrap_or("index.html");
        std::fs::write(filename, &body).map_err(|source| Error::Io {
            path: filename.into(),
            source,
        })?;
        return Ok(());
    }

    if args.silent {
        return Ok(());
    }

    write_body(&body, response, args.raw, colorize)
}

fn write_body(body: &[u8], response: &RawResponse, raw: bool, colorize: bool) -> Result<()> {
    let is_json = !raw
        && response
            .header("content-type")
            .is_some_and(|ct| ct.contains("json"));

    if is_json
        && let Ok(text) = std::str::from_utf8(body)
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
    {
        println!("{}", json_highlight::render(&value, colorize));
        return Ok(());
    }

    std::io::stdout()
        .write_all(body)
        .map_err(|source| Error::Io {
            path: "<stdout>".into(),
            source,
        })
}
