//! Artwork the desktop ships, rasterized at exactly the pixels it covers.

use crownui::prelude::{Cx, Memo, Runtime, Styled, View, image, keyed, vstack};
use serde::{Deserialize, Serialize};

use crownhome_util::raster;

/// Rasterized sizes are rounded up to this many pixels, so a widget being
/// resized is not redrawn for every pixel it grows.
const PIXEL_STEP: u32 = 8;

/// Pictures for apps on the reference board, shown instead of the theme's icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtName {
    Phone,
    Messages,
    Notes,
    Files,
    /// A blank page; the weekday and the date are drawn over it live.
    Calendar,
}

impl ArtName {
    pub fn svg(self) -> &'static [u8] {
        match self {
            Self::Phone => include_bytes!("../assets/art/phone.svg"),
            Self::Messages => include_bytes!("../assets/art/messages.svg"),
            Self::Notes => include_bytes!("../assets/art/notes.svg"),
            Self::Files => include_bytes!("../assets/art/files.svg"),
            Self::Calendar => include_bytes!("../assets/art/calendar.svg"),
        }
    }
}

pub fn pixels(points: f32, device_scale: f32) -> u32 {
    let wanted = (points * device_scale).ceil().max(1.0) as u32;
    wanted.div_ceil(PIXEL_STEP) * PIXEL_STEP
}

/// The SVG `source` picks, filling a square `edge` points wide.
pub fn picture<Source>(cx: &mut Cx, mut source: Source, edge: Memo<f32>) -> impl View + use<Source>
where
    Source: FnMut(&mut Runtime) -> &'static [u8] + 'static,
{
    let device_scale = cx.scale_factor();
    keyed(
        move |runtime| {
            let svg = source(runtime);
            (svg, pixels(edge.get(runtime), device_scale.get(runtime)))
        },
        move |_, (svg, pixels)| match raster::rasterize_svg(svg, pixels) {
            Some(picture) => image(picture.edge, picture.edge, picture.rgba)
                .w_full()
                .h_full()
                .boxed(),
            None => vstack(()).boxed(),
        },
    )
    .w(edge)
    .h(edge)
    .shrink(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_piece_of_artwork_rasterizes() {
        for name in [
            ArtName::Phone,
            ArtName::Messages,
            ArtName::Notes,
            ArtName::Files,
            ArtName::Calendar,
        ] {
            let picture = raster::rasterize_svg(name.svg(), 64).expect("rasterizes");
            assert_eq!(picture.edge, 64, "{name:?}");
        }
    }

    #[test]
    fn sizes_round_up_to_whole_steps_of_device_pixels() {
        assert_eq!(pixels(61.0, 1.0), 64);
        assert_eq!(pixels(61.0, 2.0), 128);
        assert_eq!(pixels(0.0, 1.0), 8);
    }
}
