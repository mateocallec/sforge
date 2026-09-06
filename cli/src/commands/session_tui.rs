use crate::config::Config;
use crate::error::Result;

pub fn run(config: &Config) -> Result<()> {
    crate::tui::run(config)
}
