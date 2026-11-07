//! Helpers shared by the CrownOS home crates: atomic writes, XDG paths,
//! colours and rasterising.

pub mod atomic;
pub mod color;
pub mod file_icon;
pub mod photo;
pub mod raster;
pub mod squircle;
pub mod uri;
pub mod xdg;

mod error;

pub use error::{Result, UtilError};
