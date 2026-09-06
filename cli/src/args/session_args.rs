use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// Create or update a session, or open it in $EDITOR for manual editing.
    Setup(SessionSetupArgs),
    /// List all stored sessions.
    List(SessionListArgs),
    /// Delete a session by name.
    Delete(SessionDeleteArgs),
}

#[derive(Debug, Args)]
pub struct SessionSetupArgs {
    /// Session name.
    pub name: String,

    /// Full-fidelity cookie using Set-Cookie attribute syntax, e.g.
    /// "session_id=abc; Domain=app.target.com; Path=/; Secure; HttpOnly".
    /// Repeatable. If present, replaces the session's entire cookie list.
    #[arg(long = "set-cookie", value_name = "COOKIE")]
    pub set_cookie: Vec<String>,

    /// Custom header as "Name: Value". Repeatable, stored in given order.
    /// If present, replaces the session's entire header list.
    #[arg(short = 'H', long = "header", value_name = "HEADER")]
    pub header: Vec<String>,

    /// "user:pass" for Basic (default) or Digest auth.
    #[arg(short = 'u', long = "user", value_name = "USER:PASS")]
    pub user: Option<String>,

    /// Use Digest auth instead of Basic (requires --user).
    #[arg(long, requires = "user")]
    pub digest: bool,

    /// Bearer token auth.
    #[arg(long = "bearer", value_name = "TOKEN", conflicts_with = "user")]
    pub bearer: Option<String>,
}

impl SessionSetupArgs {
    pub fn has_inline_data(&self) -> bool {
        !self.set_cookie.is_empty()
            || !self.header.is_empty()
            || self.user.is_some()
            || self.bearer.is_some()
    }
}

#[derive(Debug, Args)]
pub struct SessionListArgs {
    /// Output as JSON instead of a table.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct SessionDeleteArgs {
    /// Session name.
    pub name: String,
}
