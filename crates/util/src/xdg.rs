//! XDG base directories and where desktop entries live.

use std::path::{Path, PathBuf};

use crate::{Result, UtilError};

/// Where this app keeps its own state, `$XDG_CONFIG_HOME/crownos-home`.
pub fn state_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|config| config.join("crownos-home"))
        .ok_or(UtilError::NoConfigDirectory)
}

/// Every `applications` directory desktop entries are installed in, most
/// specific first: `$XDG_DATA_HOME`, then each of `$XDG_DATA_DIRS`.
pub fn application_dirs() -> Vec<PathBuf> {
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    dirs::data_dir()
        .into_iter()
        .chain(std::env::split_paths(&system))
        .map(|data| data.join("applications"))
        .collect()
}

/// The installed entry for a desktop id like `org.gnome.Nautilus`, or the
/// path itself when `id` already names an entry on disk.
pub fn find_desktop_entry(id: &str) -> Option<PathBuf> {
    let direct = Path::new(id);
    if direct.is_absolute() {
        return direct.is_file().then(|| direct.to_owned());
    }
    let file = format!("{}.desktop", id.trim_end_matches(".desktop"));
    application_dirs()
        .into_iter()
        .map(|dir| dir.join(&file))
        .find(|path| path.is_file())
}

/// The desktop id an entry is known by, or its full path when it is not
/// installed where [`find_desktop_entry`] would look.
pub fn desktop_id(entry: &Path) -> String {
    let installed = entry
        .parent()
        .is_some_and(|parent| application_dirs().iter().any(|dir| dir == parent));
    match entry.file_stem().and_then(|stem| stem.to_str()) {
        Some(stem) if installed => stem.to_owned(),
        _ => entry.to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_outside_the_application_dirs_is_known_by_its_path() {
        let stray = Path::new("/srv/apps/tool.desktop");
        assert_eq!(desktop_id(stray), "/srv/apps/tool.desktop");
    }

    #[test]
    fn a_missing_desktop_id_resolves_to_nothing() {
        assert_eq!(find_desktop_entry("org.crownos.DoesNotExist"), None);
        assert_eq!(find_desktop_entry("/nowhere/x.desktop"), None);
    }
}
