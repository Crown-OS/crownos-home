//! The desktop's one surface: a layer under every window, over the wallpaper.

use anyhow::{Context, Result};
use crownui::{
    AppConfig, kit,
    prelude::{Anchor, CornerShape, KeyboardInteractivity, Layer, LayerOptions, TRANSPARENT},
};

use crate::ui;
use crownhome_board::BoardSpec;

const NAMESPACE: &str = "crownos-home";
/// The size asked for until the compositor stretches the surface over the output.
const FALLBACK_SIZE: (u32, u32) = (1920, 1080);

/// `Layer::Bottom` puts the desktop directly above crownpaper's background
/// and below every window, bar and dock. An exclusive zone of `-1` ignores the
/// space panels reserve, so the board is centred on the whole output.
fn layer() -> LayerOptions {
    LayerOptions::new(
        NAMESPACE,
        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
    )
    .layer(Layer::Bottom)
    .exclusive_zone(-1)
    .keyboard(KeyboardInteractivity::None)
}

pub fn run() -> Result<()> {
    let file = BoardSpec::path()
        .map_err(|e| log::warn!("the board will not be saved: {e}"))
        .ok();
    let spec = file
        .as_deref()
        .map_or_else(BoardSpec::default, BoardSpec::load);
    let config = AppConfig::new("CrownOS Home")
        .app_id(NAMESPACE)
        .size(FALLBACK_SIZE.0, FALLBACK_SIZE.1)
        .clear_color(TRANSPARENT)
        .accessibility(false)
        .font(kit::inter_font().to_vec(), Some(kit::INTER_FAMILY))
        .layer(layer())
        .corner_shape(CornerShape::Squircle);
    crownui::run(ui::root(spec, file), config).context("the desktop stopped")
}
