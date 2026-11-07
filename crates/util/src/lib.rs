//! Helpers shared by the CrownOS home crates: atomic writes, XDG paths,
//! desktop entries, icon lookup, colours and rasterising.

pub mod atomic;
pub mod color;
pub mod desktop_entry;
pub mod gpu_preference;
pub mod icon;
pub mod raster;
pub mod squircle;
pub mod uri;
pub mod xdg;

mod error;

pub use error::{Result, UtilError};
