//! App icons from the icon theme.

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::{
    desktop_entry::DesktopEntry,
    raster::{self, Pixels},
};

const FALLBACK: &str = "application-x-executable";

static GTK_THEME: LazyLock<Option<String>> = LazyLock::new(freedesktop_icons::default_theme_gtk);

/// The icon of the app `desktop_file` describes, or the generic one.
pub fn of_app(desktop_file: &Path, edge: u32) -> Option<Pixels> {
    let named = DesktopEntry::read(desktop_file)
        .and_then(|entry| entry.icon)
        .and_then(|name| find(&name, edge));
    let file = named.or_else(|| find(FALLBACK, edge))?;
    raster::decode_file(&file, edge)
}

fn find(name: &str, edge: u32) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_owned());
    }
    let lookup = freedesktop_icons::lookup(name)
        .with_size(u16::try_from(edge).unwrap_or(u16::MAX))
        .with_cache();
    match GTK_THEME.as_deref() {
        Some(theme) => lookup.with_theme(theme).find(),
        None => lookup.find(),
    }
}
