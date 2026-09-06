use time::OffsetDateTime;

use crate::args::session_args::SessionListArgs;
use crate::config::Config;
use crate::error::Result;
use crate::output::table::Table;
use crate::session::Store;

pub fn run(config: &Config, args: SessionListArgs) -> Result<()> {
    let store = Store::new(config.sessions_dir.clone());
    let mut sessions = store.list()?;
    sessions.sort_by(|a, b| a.1.cmp(&b.1));

    if args.json {
        print_json(&store, &sessions)?;
        return Ok(());
    }

    if sessions.is_empty() {
        println!("No sessions yet. Create one with `sforge session setup \"<name>\" ...`.");
        return Ok(());
    }

    let mut table = Table::new(["NAME", "UPDATED", "COOKIES", "HEADERS"]);
    for (id, name) in &sessions {
        let session = store.load(*id)?;
        table.push([
            name.clone(),
            format_timestamp(session.meta.updated_at),
            session.cookies.len().to_string(),
            session.headers.len().to_string(),
        ]);
    }
    table.print();
    Ok(())
}

fn print_json(store: &Store, sessions: &[(uuid::Uuid, String)]) -> Result<()> {
    let mut rows = Vec::with_capacity(sessions.len());
    for (id, name) in sessions {
        let session = store.load(*id)?;
        rows.push(serde_json::json!({
            "name": name,
            "id": id.to_string(),
            "updated_at": format_timestamp(session.meta.updated_at),
            "cookies": session.cookies.len(),
            "headers": session.headers.len(),
        }));
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

fn format_timestamp(dt: OffsetDateTime) -> String {
    let format = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]");
    dt.format(&format).unwrap_or_else(|_| dt.to_string())
}
