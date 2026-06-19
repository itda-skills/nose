use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM").map_or(true, |t| t != "dumb")
            && std::io::stdout().is_terminal()
    })
}

fn paint(code: &str, s: &str) -> String {
    if s.is_empty() || !enabled() {
        s.to_string()
    } else {
        format!("\x1b[{code}m{s}\x1b[0m")
    }
}

pub(crate) fn bold(s: &str) -> String {
    paint("1", s)
}

pub(crate) fn dim(s: &str) -> String {
    paint("2", s)
}

pub(crate) fn green(s: &str) -> String {
    paint("32", s)
}

pub(crate) fn yellow(s: &str) -> String {
    paint("33", s)
}

pub(crate) fn blue(s: &str) -> String {
    paint("34", s)
}

pub(crate) fn bold_green(s: &str) -> String {
    paint("1;32", s)
}
