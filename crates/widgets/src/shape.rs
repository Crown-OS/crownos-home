//! The outline a widget draws inside its frame, for the editing handles to hug.

use crownui::prelude::{Point, Rect, Size};

/// The frame of a 2×2 tile.
pub const TILE: Size = Size::new(237.5, 241.0);
/// The corner every widget rounds to in a 2×2 tile; it grows and shrinks with the tile.
pub const ROUNDING: f32 = 36.0;

/// The corner of a widget filling a frame of `size`.
pub fn rounding(size: Size) -> f32 {
    ROUNDING * (size.width / TILE.width).min(size.height / TILE.height)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub bounds: Rect,
    pub radius: f32,
}

impl Shape {
    pub fn filling(size: Size, radius: f32) -> Self {
        Self {
            bounds: Rect::from_xywh(0.0, 0.0, size.width, size.height),
            radius,
        }
    }

    /// A full-width band `height` tall across the middle of the frame.
    pub fn band(size: Size, height: f32, radius: f32) -> Self {
        Self {
            bounds: Rect::from_xywh(0.0, (size.height - height) / 2.0, size.width, height),
            radius,
        }
    }

    /// A square `edge` wide in the middle of the frame.
    pub fn centred_square(size: Size, edge: f32, radius: f32) -> Self {
        let origin = Point::new((size.width - edge) / 2.0, (size.height - edge) / 2.0);
        Self {
            bounds: Rect::new(origin, Size::new(edge, edge)),
            radius,
        }
    }

    /// `self` grown by `gap` on every side, its corners kept concentric.
    pub fn inflate(self, gap: f32) -> Self {
        Self {
            bounds: Rect::from_xywh(
                self.bounds.min_x() - gap,
                self.bounds.min_y() - gap,
                self.bounds.size.width + 2.0 * gap,
                self.bounds.size.height + 2.0 * gap,
            ),
            radius: self.radius + gap,
        }
    }
}
