//! Starting the apps on the board.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crownhome_util::{desktop_entry::DesktopEntry, gpu_preference::GpuPreference};

use crate::non_default_gpu;

/// Starts the app `desktop_file` describes.
pub fn start(desktop_file: PathBuf) {
    match command(&desktop_file) {
        Some((argv, gpu)) => spawn(argv, gpu),
        None => log::warn!("{} has nothing to run", desktop_file.display()),
    }
}

pub fn run(argv: Vec<String>) {
    spawn(argv, GpuPreference::Default);
}

/// Runs `argv` off the UI thread. `setsid --fork` gives it a session of its
/// own and returns at once, so it outlives the desktop and is never left
/// behind as its zombie.
fn spawn(argv: Vec<String>, gpu: GpuPreference) {
    let spawned = std::thread::Builder::new()
        .name("crownos-home-launch".into())
        .spawn(move || {
            let started = Command::new("setsid")
                .arg("--fork")
                .args(&argv)
                .envs(
                    non_default_gpu::environment(gpu)
                        .iter()
                        .map(|(key, value)| (key, value)),
                )
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if let Err(e) = started {
                log::warn!("starting {argv:?}: {e}");
            }
        });
    if let Err(e) = spawned {
        log::warn!("no launcher thread: {e}");
    }
}

fn command(desktop_file: &Path) -> Option<(Vec<String>, GpuPreference)> {
    let entry = DesktopEntry::read(desktop_file)?;
    let argv = split_exec(&strip_field_codes(entry.exec.as_deref()?));
    (!argv.is_empty()).then_some((argv, entry.gpu))
}

/// Removes the `%f`, `%U`… placeholders a desktop entry's `Exec` may carry.
///
/// Nothing is substituted for them: an entry launched from the desktop is
/// launched with no arguments. `%%` is a literal percent and survives.
pub fn strip_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            // A removed code leaves the space in front of it behind.
            Some(_) => {
                if out.ends_with(' ') {
                    out.pop();
                }
            }
            None => out.push('%'),
        }
    }
    out
}

/// Splits a command line on whitespace, honouring the quoting the desktop entry
/// spec allows.
pub fn split_exec(exec: &str) -> Vec<String> {
    let mut argv = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for c in exec.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' && quote == Some('"') {
            escaped = true;
        } else if Some(c) == quote {
            quote = None;
        } else if quote.is_none() && (c == '"' || c == '\'') {
            quote = Some(c);
        } else if quote.is_none() && c.is_whitespace() {
            if !current.is_empty() {
                argv.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        argv.push(current);
    }
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_codes_are_removed_with_the_space_before_them() {
        assert_eq!(strip_field_codes("firefox %u"), "firefox");
        assert_eq!(strip_field_codes("gimp %U"), "gimp");
        assert_eq!(strip_field_codes("app %f --flag"), "app --flag");
    }

    #[test]
    fn a_doubled_percent_is_a_literal_one() {
        assert_eq!(strip_field_codes("echo 100%%"), "echo 100%");
    }

    #[test]
    fn a_trailing_percent_survives() {
        assert_eq!(strip_field_codes("app %"), "app %");
    }

    #[test]
    fn a_command_splits_on_whitespace() {
        assert_eq!(
            split_exec("firefox --new-window"),
            ["firefox", "--new-window"]
        );
    }

    #[test]
    fn quotes_hold_an_argument_together() {
        assert_eq!(
            split_exec(r#"/opt/My App/run --title "hello world""#),
            ["/opt/My", "App/run", "--title", "hello world"]
        );
        assert_eq!(split_exec("'/opt/My App/run'"), ["/opt/My App/run"]);
    }

    #[test]
    fn a_backslash_escapes_inside_double_quotes() {
        assert_eq!(split_exec(r#""a\"b""#), [r#"a"b"#]);
    }

    #[test]
    fn an_empty_command_splits_to_nothing() {
        assert!(split_exec("   ").is_empty());
    }
}
