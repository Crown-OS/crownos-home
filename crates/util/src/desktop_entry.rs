//! The parts of a `.desktop` file the board uses.

use std::path::Path;

use crate::gpu_preference::GpuPreference;

const SECTION: &str = "Desktop Entry";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopEntry {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub gpu: GpuPreference,
}

impl DesktopEntry {
    pub fn read(path: &Path) -> Option<Self> {
        let entry = freedesktop_entry_parser::parse_entry(path)
            .map_err(|e| log::debug!("{}: {e}", path.display()))
            .ok()?;
        Some(Self::from_entry(&entry))
    }

    fn from_entry(entry: &freedesktop_entry_parser::Entry) -> Self {
        let section = entry.section(SECTION);
        let field = |key: &str| section.attr(key).map(str::to_owned);
        Self {
            name: field("Name"),
            icon: field("Icon"),
            exec: field("Exec"),
            gpu: GpuPreference::from_desktop_entry(section.attr(GpuPreference::DESKTOP_ENTRY_KEY)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = "[Desktop Entry]\nName=Game\nExec=game\nPrefersNonDefaultGPU=true\n";

    fn gpu(text: &str) -> Option<GpuPreference> {
        let entry = freedesktop_entry_parser::Entry::parse(text).ok()?;
        Some(DesktopEntry::from_entry(&entry).gpu)
    }

    #[test]
    fn games_can_prefer_the_non_default_gpu() {
        assert_eq!(gpu(GAME), Some(GpuPreference::NonDefault));
        let plain = GAME.replace("PrefersNonDefaultGPU=true\n", "");
        assert_eq!(gpu(&plain), Some(GpuPreference::Default));
    }
}
