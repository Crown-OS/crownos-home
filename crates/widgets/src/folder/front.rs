//! The folder's two panes: the back with its tab on the right, and the
//! frosted front with its tab on the left, which blurs the papers behind it
//! and tips toward the viewer as they lift and spread.

use crate::shape::ROUNDING;
use crownui::{
    ext::{DrawCommand, DrawList, Paint, Path, Stroke},
    prelude::{Color, CornerRadii, Point, Rect, Size, Vector},
};

/// The front tab's share of the pane's width, and the back tab's from the
/// other side; the height of both tabs, and the run of the slope down from them.
const FRONT_TAB_SHARE: f32 = 0.44;
const BACK_TAB_SHARE: f32 = 0.52;
pub const TAB_HEIGHT: f32 = 23.0;
const TAB_SLOPE: f32 = 26.0;
const TOP_CORNER: f32 = 16.0;
pub const BOTTOM_CORNER: f32 = ROUNDING * 2.0 / 3.0;
const RIM: f32 = 1.2;
const SHADOW: (f32, f32) = (22.0, 10.0);
const FROST_BLUR: f32 = 24.0;

/// How far the front drops, how much taller it grows, how much wider its
/// bottom and its top get, fully tipped.
const TIP_DROP: f32 = 4.6;
const TIP_GROWTH: f32 = 0.045;
const TIP_WIDENING: f32 = 0.01;
const TIP_LEAN: f32 = 0.045;

/// How tinted the panes are: light over light photos, dark over dark ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Light,
    Dark,
}

impl Tone {
    pub fn of(dark: bool) -> Self {
        if dark { Self::Dark } else { Self::Light }
    }

    fn frost(self) -> Color {
        match self {
            Self::Light => Color::new(1.0, 1.0, 1.0, 0.3),
            Self::Dark => Color::new(0.1, 0.1, 0.11, 0.82),
        }
    }

    fn back(self) -> Color {
        match self {
            Self::Light => Color::new(0.92, 0.92, 0.93, 1.0),
            Self::Dark => Color::new(0.6, 0.6, 0.6, 1.0),
        }
    }

    fn rim(self) -> Color {
        match self {
            Self::Light => Color::new(1.0, 1.0, 1.0, 0.75),
            Self::Dark => Color::new(1.0, 1.0, 1.0, 0.1),
        }
    }

    pub fn ink(self) -> Color {
        match self {
            Self::Light => Color::new(0.11, 0.11, 0.12, 1.0),
            Self::Dark => Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn ink_secondary(self) -> Color {
        self.ink().with_alpha_factor(0.55)
    }
}

/// How far the front has tipped toward the viewer: 0 upright, 1 fully.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tip(pub f32);

impl Tip {
    /// How far down the front has dropped, in design points.
    pub fn drop(self) -> f32 {
        TIP_DROP * (2.0 * self.0).min(1.0)
    }

    /// Where `(x, y)` of an upright pane of `size` lands once tipped.
    fn bend(self, size: Size, factor: f32, x: f32, y: f32) -> Point {
        let (width, height) = (size.width, size.height);
        let toward_top = 1.0 - y / height;
        let widening = 1.0 + self.0 * (TIP_WIDENING + TIP_LEAN * toward_top);
        Point::new(
            width / 2.0 + (x - width / 2.0) * widening,
            height + (y - height) * (1.0 + TIP_GROWTH * self.0) + self.drop() * factor,
        )
    }
}

/// A pane's outline in a box of `size`, `factor` design points to the point,
/// with a tab `tab_share` of the width long on its left; `at` places each point.
fn outline(size: Size, factor: f32, tab_share: f32, at: impl Fn(f32, f32) -> Point) -> Path {
    let (width, height) = (size.width, size.height);
    let (tab, slope, top, bottom) = (
        TAB_HEIGHT * factor,
        TAB_SLOPE * factor,
        TOP_CORNER * factor,
        BOTTOM_CORNER * factor,
    );
    let tab_end = width * tab_share;
    Path::new()
        .move_to(at(0.0, top))
        .quad_to(at(0.0, 0.0), at(top, 0.0))
        .line_to(at(tab_end - slope / 2.0, 0.0))
        .cubic_to(
            at(tab_end, 0.0),
            at(tab_end, tab),
            at(tab_end + slope / 2.0, tab),
        )
        .line_to(at(width - top, tab))
        .quad_to(at(width, tab), at(width, tab + top))
        .line_to(at(width, height - bottom))
        .quad_to(at(width, height), at(width - bottom, height))
        .line_to(at(bottom, height))
        .quad_to(at(0.0, height), at(0.0, height - bottom))
        .close()
}

/// The front's outline in a box of `size`, tipped by `tip`.
pub fn front_outline(size: Size, factor: f32, tip: Tip) -> Path {
    outline(size, factor, FRONT_TAB_SHARE, |x, y| {
        tip.bend(size, factor, x, y)
    })
}

fn back_outline(size: Size, factor: f32) -> Path {
    outline(size, factor, BACK_TAB_SHARE, |x, y| {
        Point::new(size.width - x, y)
    })
}

/// The back pane, which the papers stand in front of, reaching wherever the
/// front tips to so the front frosts nothing but the back and the papers.
/// The front's box is `front_offset` below the back's and `front_size` big.
pub fn paint_back(
    list: &mut DrawList,
    size: Size,
    factor: f32,
    tone: Tone,
    front: (Vector, Size, Tip),
) {
    let path = back_outline(size, factor);
    let (front_offset, front_size, tip) = front;
    let fill = Paint::solid(tone.back());
    list.push(DrawCommand::FillPath(path.clone(), fill.clone()));
    list.save();
    list.translate(front_offset);
    list.push(DrawCommand::FillPath(
        front_outline(front_size, factor, tip),
        fill,
    ));
    list.restore();
    list.push(DrawCommand::StrokePath(
        path,
        Stroke::new(RIM * factor),
        Paint::solid(tone.rim()),
    ));
}

/// The front pane, frosting whatever is behind it.
pub fn paint_front(list: &mut DrawList, size: Size, factor: f32, tone: Tone, tip: Tip) {
    let path = front_outline(size, factor, tip);
    let bounds = path.bounds();
    let (blur, drop) = SHADOW;
    let (top_left, bottom_right) = (
        tip.bend(size, factor, 0.0, TAB_HEIGHT * factor),
        tip.bend(size, factor, size.width, size.height),
    );
    let body = Rect::from_xywh(
        size.width - bottom_right.x,
        top_left.y,
        2.0 * bottom_right.x - size.width,
        bottom_right.y - top_left.y,
    );
    list.shadow(
        body,
        CornerRadii::uniform(BOTTOM_CORNER * factor),
        Color::new(0.0, 0.0, 0.0, 0.14),
        blur * factor,
        Vector::new(0.0, drop * factor),
    );
    list.save();
    list.push(DrawCommand::ClipPath(path.clone()));
    list.push(DrawCommand::SaveLayer {
        bounds: Some(bounds),
        opacity: 1.0,
        backdrop_blur: Some(FROST_BLUR * factor),
    });
    list.restore();
    list.push(DrawCommand::FillPath(
        path.clone(),
        Paint::solid(tone.frost()),
    ));
    list.restore();
    list.push(DrawCommand::StrokePath(
        path,
        Stroke::new(RIM * factor),
        Paint::solid(tone.rim()),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size::new(200.0, 150.0);

    #[test]
    fn the_tabs_stand_on_opposite_sides() {
        let front = front_outline(SIZE, 1.0, Tip(0.0)).bounds();
        let back = back_outline(SIZE, 1.0).bounds();
        assert_eq!(front, back);
        assert!((front.max_y() - SIZE.height).abs() < 0.01);
    }

    #[test]
    fn a_tipped_front_drops_and_widens_most_at_the_top() {
        let upright = Tip(0.0);
        let tipped = Tip(1.0);
        let top = tipped.bend(SIZE, 1.0, 0.0, 0.0);
        let bottom = tipped.bend(SIZE, 1.0, 0.0, SIZE.height);
        assert_eq!(upright.bend(SIZE, 1.0, 0.0, 0.0), Point::new(0.0, 0.0));
        assert!(top.x < bottom.x && bottom.x < 0.0);
        assert!((bottom.y - SIZE.height - TIP_DROP).abs() < 0.01);
    }
}
