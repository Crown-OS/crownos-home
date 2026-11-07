use crownhome_util::UtilError;

pub type Result<T, E = BoardError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum BoardError {
    #[error(transparent)]
    Storage(#[from] UtilError),

    #[error("encoding the board")]
    Encode(#[from] toml::ser::Error),

    #[error("settings of the `{kind}` widget")]
    WidgetSettings {
        kind: String,
        #[source]
        source: toml::de::Error,
    },
}
