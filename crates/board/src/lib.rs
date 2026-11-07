//! The CrownOS home board: the grid widgets snap to, the arrangement saved in
//! `widgets.toml`, and the rules for where a widget may go.

pub mod grid;
pub mod spec;
pub mod tile;

mod error;

pub use error::{BoardError, Result};
pub use grid::{Area, BoardGrid, GridSpec};
pub use spec::{BoardSpec, WidgetSpec};
pub use tile::{Tile, TileId};
