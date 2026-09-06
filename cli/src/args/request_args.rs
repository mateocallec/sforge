use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub struct RequestArgs {
    /// URL to request. Required unless a subcommand (e.g. `session`) is
    /// given instead.
    pub url: Option<String>,

    /// HTTP method. Defaults to GET, or POST when a body is given.
    #[arg(short = 'X', long = "request", value_name = "METHOD")]
    pub method: Option<String>,

    /// Attach a named session's cookies/headers/auth to this request.
    #[arg(long = "session", value_name = "NAME")]
    pub session: Option<String>,

    /// Custom header "Name: Value". Repeatable, sent in given order.
    /// "Name:" (nothing after the colon) suppresses that header entirely,
    /// even a default sforge would otherwise add; "Name;" sends it empty.
    #[arg(short = 'H', long = "header", value_name = "HEADER")]
    pub header: Vec<String>,

    #[arg(short = 'A', long = "user-agent", value_name = "UA")]
    pub user_agent: Option<String>,

    #[arg(short = 'e', long = "referer", value_name = "URL")]
    pub referer: Option<String>,

    /// Send Accept-Encoding and transparently decompress the response for
    /// display.
    #[arg(long = "compressed")]
    pub compressed: bool,

    /// "name=value; name2=value2", added to any session cookies already
    /// matching this request's host/path. No domain scoping — applies to
    /// this request's own host only.
    #[arg(short = 'b', long = "cookie", value_name = "DATA")]
    pub cookie: Option<String>,

    /// Full-fidelity cookie using Set-Cookie attribute syntax. Repeatable,
    /// matched against the request URL the same way session cookies are.
    #[arg(long = "set-cookie", value_name = "COOKIE")]
    pub set_cookie: Vec<String>,

    /// "user:pass" for Basic (default) or Digest auth.
    #[arg(short = 'u', long = "user", value_name = "USER:PASS")]
    pub user: Option<String>,

    #[arg(long = "digest", requires = "user")]
    pub digest: bool,

    #[arg(long = "bearer", value_name = "TOKEN", conflicts_with = "user")]
    pub bearer: Option<String>,

    /// Repeatable, joined with '&'. "@file" reads the value from a file,
    /// "@-" from stdin. Sets Content-Type: application/x-www-form-urlencoded
    /// by default and implies POST.
    #[arg(short = 'd', long = "data", value_name = "DATA")]
    pub data: Vec<String>,

    /// Like -d, but "@" is never special — always a literal value.
    #[arg(long = "data-raw", value_name = "DATA")]
    pub data_raw: Vec<String>,

    /// Like -d, but bytes are sent exactly as given (no CR/LF stripping).
    #[arg(long = "data-binary", value_name = "DATA")]
    pub data_binary: Option<String>,

    /// "name=value", "=value", "name@file", or "@file" — each part is
    /// URL-encoded before being joined into the body.
    #[arg(long = "data-urlencode", value_name = "DATA")]
    pub data_urlencode: Vec<String>,

    /// JSON body, or "@file". Sets Content-Type and Accept to
    /// application/json and implies POST.
    #[arg(long = "json", value_name = "DATA")]
    pub json: Option<String>,

    /// "name=value", "name=@file[;type=...][;filename=...]" (upload), or
    /// "name=<file" (use the file's contents as the field's text value).
    /// Repeatable; builds a multipart/form-data body.
    #[arg(short = 'F', long = "form", value_name = "FIELD")]
    pub form: Vec<String>,

    /// Send this file's contents as the request body.
    #[arg(short = 'T', long = "upload-file", value_name = "FILE")]
    pub upload_file: Option<PathBuf>,

    /// Force Transfer-Encoding: chunked even though the body's length is
    /// known upfront — a parser-differential test curl doesn't expose.
    #[arg(long = "chunked")]
    pub chunked: bool,

    /// Turn -d/--data into a query string instead of a body (GET-style).
    #[arg(short = 'G', long = "get")]
    pub get: bool,

    #[arg(short = 'k', long = "insecure")]
    pub insecure: bool,

    #[arg(long = "cacert", value_name = "FILE")]
    pub cacert: Option<PathBuf>,

    #[arg(long = "cert", value_name = "FILE", requires = "key")]
    pub cert: Option<PathBuf>,

    #[arg(long = "key", value_name = "FILE", requires = "cert")]
    pub key: Option<PathBuf>,

    /// "[scheme://][user:pass@]host:port". scheme is http (default) or
    /// socks5/socks5h (socks5h resolves the target hostname on the proxy
    /// side instead of locally).
    #[arg(short = 'x', long = "proxy", value_name = "URL")]
    pub proxy: Option<String>,

    /// "host:port:address" — connect to `address` instead of resolving
    /// `host`, while Host/SNI still carry `host`. Repeatable.
    #[arg(long = "resolve", value_name = "HOST:PORT:ADDRESS")]
    pub resolve: Vec<String>,

    #[arg(short = 'i', long = "include")]
    pub include: bool,

    #[arg(short = 'I', long = "head")]
    pub head: bool,

    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Save the body to a file named after the URL's last path segment.
    #[arg(short = 'O', long = "remote-name")]
    pub remote_name: bool,

    #[arg(short = 'D', long = "dump-header", value_name = "FILE")]
    pub dump_header: Option<PathBuf>,

    /// Print the exact bytes sent and received (curl-style '>'/'<' lines).
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    #[arg(short = 's', long = "silent")]
    pub silent: bool,

    #[arg(
        long = "color",
        value_name = "auto|always|never",
        default_value = "auto"
    )]
    pub color: String,

    /// Skip JSON reformatting/highlighting — print the body exactly as
    /// received (after --compressed decompression, if given).
    #[arg(long = "raw")]
    pub raw: bool,

    #[arg(short = 'm', long = "max-time", value_name = "SECONDS")]
    pub max_time: Option<f64>,

    #[arg(long = "connect-timeout", value_name = "SECONDS")]
    pub connect_timeout: Option<f64>,

    #[arg(long = "retry", value_name = "N", default_value_t = 0)]
    pub retry: u32,

    #[arg(long = "retry-delay", value_name = "SECONDS", default_value_t = 1)]
    pub retry_delay: u64,

    #[arg(short = 'L', long = "location")]
    pub location: bool,

    #[arg(long = "max-redirs", value_name = "N", default_value_t = 50)]
    pub max_redirs: u32,
}
