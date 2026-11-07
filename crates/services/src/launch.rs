//! Running commands from the desktop.

use std::process::{Command, Stdio};

/// Runs `argv` off the UI thread. `setsid --fork` gives it a session of its
/// own and returns at once, so it outlives the desktop and is never left
/// behind as its zombie.
pub fn run(argv: Vec<String>) {
    let spawned = std::thread::Builder::new()
        .name("crownos-home-launch".into())
        .spawn(move || {
            let started = Command::new("setsid")
                .arg("--fork")
                .args(&argv)
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
