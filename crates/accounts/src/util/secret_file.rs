//! Writing a file only its owner can read, without ever leaving half of it.

use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::Path};

use crate::{AccountError, Result};

const OWNER_ONLY: u32 = 0o600;

/// Writes `contents` to `path` through a sibling temp file created `0600`,
/// then renames it over the old one.
pub fn write(path: &Path, contents: &str) -> Result<()> {
    let failed = |path: &Path| {
        let path = path.to_owned();
        move |source| AccountError::Write { path, source }
    };
    let directory = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(directory).map_err(failed(directory))?;
    let temporary = path.with_extension(format!("tmp{}", std::process::id()));
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(OWNER_ONLY)
        .open(&temporary)
        .and_then(|mut file| file.write_all(contents.as_bytes()))
        .map_err(failed(&temporary))?;
    std::fs::rename(&temporary, path).map_err(|source| {
        let _ = std::fs::remove_file(&temporary);
        failed(path)(source)
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn the_file_is_readable_by_its_owner_alone() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("crownos-accounts-{}", std::process::id()));
        let path = dir.join("spotify.toml");
        write(&path, "id = \"x\"")?;
        let mode = std::fs::metadata(&path)
            .map(|metadata| metadata.permissions().mode() & 0o777)
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(mode, OWNER_ONLY);
        Ok(())
    }
}
