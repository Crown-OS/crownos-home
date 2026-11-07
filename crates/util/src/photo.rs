//! Photos cut down to the pictures a widget draws of them.

use image::{RgbaImage, imageops};

use crate::raster::Picture;

/// Pixels across the haze; it is drawn stretched, so it only has to hold colour.
const HAZE_WIDTH: u32 = 48;
const HAZE_SIGMA: f32 = 3.0;
/// Below this average brightness a photo counts as dark.
const DARK_BELOW: f32 = 0.45;

/// A photo, as drawn on a card, and what a frosted pane over it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Print {
    pub picture: Picture,
    pub haze: Picture,
    pub dark: bool,
}

/// `bytes` decoded, cropped to the aspect of `width` × `height` around its
/// centre, and scaled to fill it.
pub fn print(bytes: &[u8], width: u32, height: u32) -> Option<Print> {
    let source = image::load_from_memory(bytes)
        .map_err(|e| log::debug!("a photo that does not decode: {e}"))
        .ok()?;
    let filled = source
        .resize_to_fill(width, height, imageops::FilterType::Triangle)
        .into_rgba8();
    let haze_height = (HAZE_WIDTH * height / width.max(1)).max(1);
    let small = imageops::resize(
        &filled,
        HAZE_WIDTH,
        haze_height,
        imageops::FilterType::Triangle,
    );
    let haze = imageops::fast_blur(&small, HAZE_SIGMA);
    Some(Print {
        dark: brightness(&small) < DARK_BELOW,
        haze: picture(haze),
        picture: picture(filled),
    })
}

fn picture(image: RgbaImage) -> Picture {
    Picture {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

/// The mean of each pixel's Rec. 709 luma, from 0 to 1.
fn brightness(image: &RgbaImage) -> f32 {
    let pixels = image.pixels().len().max(1) as f32;
    image
        .pixels()
        .map(|pixel| {
            let [red, green, blue, _] = pixel.0.map(f32::from);
            (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255.0
        })
        .sum::<f32>()
        / pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba};

    fn encoded(color: [u8; 4]) -> Result<Vec<u8>, image::ImageError> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        RgbaImage::from_pixel(30, 20, Rgba(color)).write_to(&mut bytes, ImageFormat::Png)?;
        Ok(bytes.into_inner())
    }

    #[test]
    fn a_print_fills_the_size_asked_for() -> Result<(), image::ImageError> {
        let print = print(&encoded([250, 250, 250, 255])?, 12, 8).expect("decodes");
        assert_eq!((print.picture.width, print.picture.height), (12, 8));
        assert_eq!(print.haze.width, HAZE_WIDTH);
        assert!(!print.dark);
        Ok(())
    }

    #[test]
    fn a_black_photo_is_dark() -> Result<(), image::ImageError> {
        let print = print(&encoded([10, 10, 10, 255])?, 12, 8).expect("decodes");
        assert!(print.dark);
        Ok(())
    }
}
