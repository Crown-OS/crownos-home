use std::{
    cmp::Reverse,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// How many of the newest files a folder shows.
pub const SHOWN: usize = 3;
/// What the photo decoder reads.
const PHOTO_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A picture the photo decoder reads.
    Photo,
    Folder,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: Kind,
}

/// A folder's visible contents: how many there are and the newest few.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub count: usize,
    pub newest: Vec<Entry>,
    /// When the folder itself last changed, to tell whether to read it again.
    pub changed: Option<SystemTime>,
}

fn is_photo(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            PHOTO_EXTENSIONS
                .iter()
                .any(|photo| extension.eq_ignore_ascii_case(photo))
        })
}

pub fn changed(dir: &Path) -> Option<SystemTime> {
    fs::metadata(dir)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// What is in `dir`, hidden files left out, newest first.
pub fn read(dir: &Path) -> io::Result<Listing> {
    let mut entries: Vec<(SystemTime, Entry)> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter_map(|item| {
            let name = item.file_name().into_string().ok()?;
            if name.starts_with('.') {
                return None;
            }
            let modified = item.metadata().and_then(|meta| meta.modified()).ok()?;
            let path = item.path();
            let kind = match item.file_type().ok()?.is_dir() {
                true => Kind::Folder,
                false if is_photo(&path) => Kind::Photo,
                false => Kind::File,
            };
            Some((modified, Entry { path, name, kind }))
        })
        .collect();
    let count = entries.len();
    entries.sort_unstable_by_key(|(modified, _)| Reverse(*modified));
    Ok(Listing {
        count,
        newest: entries
            .into_iter()
            .take(SHOWN)
            .map(|(_, entry)| entry)
            .collect(),
        changed: changed(dir),
    })
}

/// A name for `file` in `dir` that nothing there has yet: `photo.jpg`, then
/// `photo 2.jpg`, `photo 3.jpg`…
pub fn free_name(dir: &Path, file: &Path) -> Option<PathBuf> {
    let stem = file.file_stem()?.to_string_lossy();
    let extension = file
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|copy| match copy {
            1 => dir.join(format!("{stem}{extension}")),
            copy => dir.join(format!("{stem} {copy}{extension}")),
        })
        .find(|candidate| !candidate.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> io::Result<PathBuf> {
        let dir =
            std::env::temp_dir().join(format!("crownhome-folder-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn hidden_files_are_left_out_and_photos_are_known() -> io::Result<()> {
        let dir = scratch("listing")?;
        for name in [".hidden", "notes.txt", "Beach.JPG"] {
            fs::write(dir.join(name), b"x")?;
        }
        let listing = read(&dir)?;
        assert_eq!(listing.count, 2);
        assert!(
            listing
                .newest
                .iter()
                .any(|entry| entry.name == "Beach.JPG" && entry.kind == Kind::Photo)
        );
        assert!(
            listing
                .newest
                .iter()
                .any(|entry| entry.name == "notes.txt" && entry.kind == Kind::File)
        );
        fs::remove_dir_all(dir)
    }

    #[test]
    fn a_taken_name_gets_a_number() -> io::Result<()> {
        let dir = scratch("names")?;
        fs::write(dir.join("photo.jpg"), b"x")?;
        assert_eq!(
            free_name(&dir, Path::new("/elsewhere/photo.jpg")),
            Some(dir.join("photo 2.jpg"))
        );
        assert_eq!(
            free_name(&dir, Path::new("/elsewhere/new.png")),
            Some(dir.join("new.png"))
        );
        fs::remove_dir_all(dir)
    }
}
