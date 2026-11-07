//! Decoding pictures straight to the pixels they are drawn at.

use image::imageops;
use resvg::{tiny_skia, usvg};

/// A square picture in straight-alpha RGBA8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub edge: u32,
    pub rgba: Vec<u8>,
}

/// A picture of any shape in straight-alpha RGBA8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl From<Pixels> for Picture {
    fn from(square: Pixels) -> Self {
        Self {
            width: square.edge,
            height: square.edge,
            rgba: square.rgba,
        }
    }
}

/// How an SVG is laid into a frame of another shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    /// Whole, centred, leaving the rest clear.
    Contain,
    /// Filling the frame, centred, its overflow cropped.
    Cover,
}

fn render_svg(bytes: &[u8], width: u32, height: u32, fit: Fit) -> Option<Picture> {
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|e| log::warn!("an svg that does not parse: {e}"))
        .ok()?;
    let size = tree.size();
    let (across, down) = (
        width as f32 / size.width().max(1.0),
        height as f32 / size.height().max(1.0),
    );
    let scale = match fit {
        Fit::Contain => across.min(down),
        Fit::Cover => across.max(down),
    };
    let offset_x = (width as f32 - size.width() * scale) / 2.0;
    let offset_y = (height as f32 - size.height() * scale) / 2.0;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let transform =
        tiny_skia::Transform::from_scale(scale, scale).post_translate(offset_x, offset_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let straight = pixel.demultiply();
            [
                straight.red(),
                straight.green(),
                straight.blue(),
                straight.alpha(),
            ]
        })
        .collect();
    Some(Picture {
        width,
        height,
        rgba,
    })
}

pub fn rasterize_svg(bytes: &[u8], edge: u32) -> Option<Pixels> {
    render_svg(bytes, edge, edge, Fit::Contain).map(|picture| Pixels {
        edge,
        rgba: picture.rgba,
    })
}

/// The SVG filling `width` × `height` pixels, centred and cropped like a backdrop.
pub fn rasterize_svg_covering(bytes: &[u8], width: u32, height: u32) -> Option<Picture> {
    render_svg(bytes, width, height, Fit::Cover)
}

/// A picture in any format the crate decodes, cropped to its centre square
/// and scaled to `edge` × `edge` pixels, as album art is shown.
pub fn cover_square(bytes: &[u8], edge: u32) -> Option<Pixels> {
    let source = image::load_from_memory(bytes)
        .map_err(|e| log::debug!("a picture that does not decode: {e}"))
        .ok()?;
    let side = source.width().min(source.height());
    let square = source.crop_imm(
        (source.width() - side) / 2,
        (source.height() - side) / 2,
        side,
        side,
    );
    let rgba = square
        .resize_exact(edge, edge, imageops::FilterType::Triangle)
        .into_rgba8()
        .into_raw();
    Some(Pixels { edge, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbaImage};

    const TALL_RED: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="20" fill="#ff0000"/></svg>"##;

    #[test]
    fn a_wide_picture_is_cropped_to_its_centre() -> Result<(), image::ImageError> {
        let mut wide = RgbaImage::from_pixel(4, 2, image::Rgba([0, 0, 255, 255]));
        wide.put_pixel(1, 0, image::Rgba([255, 0, 0, 255]));
        let mut encoded = std::io::Cursor::new(Vec::new());
        wide.write_to(&mut encoded, ImageFormat::Png)?;
        let cover = cover_square(encoded.get_ref(), 2).expect("decodes");
        assert_eq!(cover.rgba.len(), 2 * 2 * 4);
        assert_eq!(&cover.rgba[..4], &[255, 0, 0, 255]);
        Ok(())
    }

    #[test]
    fn a_covering_svg_fills_a_wide_frame() {
        let picture = rasterize_svg_covering(TALL_RED, 8, 4).expect("rasterizes");
        assert_eq!(picture.rgba.len(), 8 * 4 * 4);
        assert!(picture.rgba.chunks(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn a_tall_svg_is_centred_in_its_square() {
        let pixels = rasterize_svg(TALL_RED, 8).expect("rasterizes");
        assert_eq!(pixels.rgba.len(), 8 * 8 * 4);
        let alpha = |x: usize, y: usize| pixels.rgba[(y * 8 + x) * 4 + 3];
        assert_eq!(alpha(0, 4), 0);
        assert_eq!(alpha(4, 4), 255);
    }
}
