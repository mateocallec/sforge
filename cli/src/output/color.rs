use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "auto" => Some(ColorMode::Auto),
            "always" => Some(ColorMode::Always),
            "never" => Some(ColorMode::Never),
            _ => None,
        }
    }
}

/// `--color` wins when given explicitly; `auto` otherwise respects
/// `NO_COLOR` and falls back to disabling color on a non-TTY stdout (e.g.
/// piped into another command).
pub fn enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
        }
    }
}
