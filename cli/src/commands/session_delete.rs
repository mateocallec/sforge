use crate::args::session_args::SessionDeleteArgs;
use crate::config::Config;
use crate::error::Result;
use crate::session::Store;

pub fn run(config: &Config, args: SessionDeleteArgs) -> Result<()> {
    let store = Store::new(config.sessions_dir.clone());
    store.delete(&args.name)?;
    println!("Deleted session \"{}\".", args.name);
    Ok(())
}
