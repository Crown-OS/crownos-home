//! Writing a file without ever leaving a half-written one behind.

use std::path::Path;

use crate::{Result, UtilError};

/// Writes `contents` to `path` by way of a sibling temp file and a rename.
///
/// `rename(2)` within a directory is atomic, so a reader either sees the old
/// file or the new one. The temp file is a sibling rather than in `/tmp` because
/// a rename across filesystems is not a rename at all.
pub fn write(path: &Path, contents: &str) -> Result<()> {
    let directory = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(directory).map_err(|e| UtilError::write(directory, e))?;

    let temporary = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&temporary, contents).map_err(|e| UtilError::write(&temporary, e))?;

    std::fs::rename(&temporary, path).map_err(|e| {
        let _ = std::fs::remove_file(&temporary);
        UtilError::write(path, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_creates_the_directory_and_the_file() {
        let dir = std::env::temp_dir().join("crownos-home-atomic/nested");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap_or(Path::new(".")));
        let path = dir.join("layout.toml");

        write(&path, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");

        write(&path, "replaced").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "replaced");

        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() != "layout.toml")
            .collect();
        assert!(leftovers.is_empty(), "temp file was left behind");

        let _ = std::fs::remove_dir_all(dir.parent().unwrap_or(Path::new(".")));
    }
}
