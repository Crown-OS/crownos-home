use crownhome_board::BoardError;

pub type Result<T, E = WidgetError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum WidgetError {
    #[error("no widget kind is called `{0}`")]
    UnknownWidget(String),

    #[error(transparent)]
    Settings(#[from] BoardError),
}
