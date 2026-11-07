//! XDG base directories.

use std::path::PathBuf;

use crate::{Result, UtilError};

/// Where this app keeps its own state, `$XDG_CONFIG_HOME/crownos-home`.
pub fn state_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|config| config.join("crownos-home"))
        .ok_or(UtilError::NoConfigDirectory)
}
