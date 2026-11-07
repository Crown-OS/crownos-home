//! Decoding pictures straight to the pixels they are drawn at.

use std::path::Path;

use image::{ImageFormat, RgbaImage, imageops};
use resvg::{tiny_skia, usvg};

/// A square picture in straight-alpha RGBA8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub edge: u32,
    pub rgba: Vec<u8>,
}

/// An SVG or PNG file, fitted and centred in `edge` × `edge` pixels.
pub fn decode_file(path: &Path, edge: u32) -> Option<Pixels> {
    let bytes = std::fs::read(path)
        .map_err(|e| log::debug!("{}: {e}", path.display()))
        .ok()?;
    let is_svg = path
        .extension()
        .is_some_and(|extension| extension == "svg" || extension == "svgz");
    if is_svg {
        rasterize_svg(&bytes, edge)
    } else {
        fit_png(&bytes, edge)
    }
}

pub fn rasterize_svg(bytes: &[u8], edge: u32) -> Option<Pixels> {
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
        .map_err(|e| log::warn!("an svg that does not parse: {e}"))
        .ok()?;
    let size = tree.size();
    let scale = edge as f32 / size.width().max(size.height()).max(1.0);
    let offset_x = (edge as f32 - size.width() * scale) / 2.0;
    let offset_y = (edge as f32 - size.height() * scale) / 2.0;
    let mut pixmap = tiny_skia::Pixmap::new(edge, edge)?;
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
    Some(Pixels { edge, rgba })
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

fn fit_png(bytes: &[u8], edge: u32) -> Option<Pixels> {
    let source = image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .map_err(|e| log::debug!("an icon that does not decode: {e}"))
        .ok()?
        .into_rgba8();
    let (width, height) = source.dimensions();
    let scale = edge as f32 / width.max(height).max(1) as f32;
    let fitted = |length: u32| ((length as f32 * scale).round() as u32).clamp(1, edge);
    let (fit_width, fit_height) = (fitted(width), fitted(height));
    let resized = if (fit_width, fit_height) == (width, height) {
        source
    } else {
        imageops::resize(
            &source,
            fit_width,
            fit_height,
            imageops::FilterType::Triangle,
        )
    };
    let mut canvas = RgbaImage::new(edge, edge);
    imageops::overlay(
        &mut canvas,
        &resized,
        i64::from((edge - fit_width) / 2),
        i64::from((edge - fit_height) / 2),
    );
    Some(Pixels {
        edge,
        rgba: canvas.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_tall_svg_is_centred_in_its_square() {
        let pixels = rasterize_svg(TALL_RED, 8).expect("rasterizes");
        assert_eq!(pixels.rgba.len(), 8 * 8 * 4);
        let alpha = |x: usize, y: usize| pixels.rgba[(y * 8 + x) * 4 + 3];
        assert_eq!(alpha(0, 4), 0);
        assert_eq!(alpha(4, 4), 255);
    }
}
