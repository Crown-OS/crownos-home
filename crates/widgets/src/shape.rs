//! The outline a widget draws inside its frame, for the editing handles to hug.

use crownui::prelude::{Point, Rect, Size};

pub const DEFAULT_ROUNDING: f32 = 28.0;

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
