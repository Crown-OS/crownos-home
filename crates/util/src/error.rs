use std::path::PathBuf;

pub type Result<T, E = UtilError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum UtilError {
    #[error("no configuration directory: $XDG_CONFIG_HOME and $HOME are both unset")]
    NoConfigDirectory,

    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl UtilError {
    pub fn write(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }
}
