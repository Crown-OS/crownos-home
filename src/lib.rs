//! The CrownOS desktop: live glass widgets on a grid above the wallpaper,
//! drawn with crownui on a layer surface under every window.
//!
//! The arrangement lives in [`crownhome_board`], the widget views in
//! [`crownhome_widgets`] and what they read in [`crownhome_services`]; this
//! crate is the desktop around them — the context menu, the gallery and the
//! editing gestures.

mod app;
pub mod ui;

pub use app::run;
pub use crownhome_board as board;
