//! File-type pictures from the CrownOS icon theme, chosen by what a file's
//! name says it holds.

use std::path::{Path, PathBuf};

const THEME: &str = "CrownOS-light";
const FOLDER: &str = "folder";
const UNKNOWN: &str = "unknown";

/// The theme's names for `file`, the most specific first.
fn names(file: &Path) -> Vec<String> {
    let mut names: Vec<String> = mime_guess::from_path(file)
        .iter()
        .flat_map(|mime| {
            [
                mime.essence_str().replace('/', "-"),
                format!("{}-x-generic", mime.type_()),
            ]
        })
        .collect();
    names.push(UNKNOWN.to_owned());
    names
}

fn lookup(name: &str, edge: u16) -> Option<PathBuf> {
    freedesktop_icons::lookup(name)
        .with_theme(THEME)
        .with_size(edge)
        .force_svg()
        .with_cache()
        .find()
}

/// The SVG picturing `file`, or a folder, about `edge` pixels wide.
pub fn find(file: &Path, folder: bool, edge: u16) -> Option<PathBuf> {
    if folder {
        return lookup(FOLDER, edge);
    }
    names(file).iter().find_map(|name| lookup(name, edge))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_named_by_its_type_then_its_kind_then_unknown() {
        assert_eq!(
            names(Path::new("Report.pdf")),
            ["application-pdf", "application-x-generic", "unknown"]
        );
        assert_eq!(names(Path::new("no-extension")), ["unknown"]);
    }
}
