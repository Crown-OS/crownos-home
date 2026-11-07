//! A folder on disk: what is newest in it, opening it and its files, and
//! copying picked files into it.

mod chooser;
mod listing;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crownui::prelude::{Cx, Signal};

pub use listing::{Entry, Kind, Listing, SHOWN};

use crate::launch;
use crownhome_util::{
    file_icon,
    photo::{self, Print},
    raster::{self, Pixels},
};

/// How often the folder is checked for changes.
const POLL: Duration = Duration::from_secs(3);

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(work).await.ok()
}

/// `~/Pictures` and the like, with the home directory filled in.
pub fn expand(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// Keeps `shown` on what `dir` holds, reading it again only when it changes.
pub fn follow(cx: &mut Cx, dir: PathBuf, shown: Signal<Option<Listing>>) {
    let known = shown.with(cx, |listing| {
        listing.as_ref().and_then(|listing| listing.changed)
    });
    let reading = dir.clone();
    cx.spawn(
        blocking(move || {
            let changed = listing::changed(&reading);
            (changed.is_none() || changed != known).then(|| listing::read(&reading))
        }),
        move |cx, read| {
            match read.flatten() {
                Some(Ok(listing)) => shown.set(cx, Some(listing)),
                Some(Err(e)) => {
                    log::debug!("folder {}: {e}", dir.display());
                    shown.set(cx, None);
                }
                None => {}
            }
            cx.spawn(
                async move { tokio::time::sleep(POLL).await },
                move |cx, ()| follow(cx, dir, shown),
            );
        },
    );
}

/// What `dir` holds now.
pub async fn read(dir: PathBuf) -> Option<Listing> {
    blocking(move || listing::read(&dir).ok()).await.flatten()
}

/// The photo at `path` cut to `width` × `height` pixels.
pub async fn print(path: PathBuf, width: u32, height: u32) -> Option<Print> {
    blocking(move || photo::print(&std::fs::read(path).ok()?, width, height))
        .await
        .flatten()
}

/// The icon theme's picture of what `path` holds, `edge` pixels square.
pub async fn icon(path: PathBuf, folder: bool, edge: u32) -> Option<Pixels> {
    blocking(move || {
        let size = u16::try_from(edge).unwrap_or(u16::MAX);
        let svg = std::fs::read(file_icon::find(&path, folder, size)?).ok()?;
        raster::rasterize_svg(&svg, edge)
    })
    .await
    .flatten()
}

/// Opens a file or folder in the app the desktop picks for it.
pub fn open(path: &Path) {
    launch::run(vec![
        "xdg-open".to_owned(),
        path.to_string_lossy().into_owned(),
    ]);
}

/// Asks for files and copies them into `dir`, returning the names they arrived under.
pub async fn add_files(dir: PathBuf) -> Vec<String> {
    let title = format!(
        "Add to {}",
        dir.file_name()
            .map_or_else(|| dir.to_string_lossy(), |name| name.to_string_lossy())
    );
    match chooser::pick_files(&title).await {
        Ok(picked) => copy_in(dir, picked).await,
        Err(e) => {
            log::warn!("choosing files: {e}");
            Vec::new()
        }
    }
}

/// Copies `files` into `dir` without replacing anything there, returning
/// the names they arrived under.
pub async fn copy_in(dir: PathBuf, files: Vec<PathBuf>) -> Vec<String> {
    let mut arrived = Vec::with_capacity(files.len());
    for file in files {
        let Some(target) = listing::free_name(&dir, &file) else {
            continue;
        };
        match tokio::fs::copy(&file, &target).await {
            Ok(_) => arrived.extend(
                target
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
            ),
            Err(e) => log::warn!("copying {} into {}: {e}", file.display(), dir.display()),
        }
    }
    arrived
}
