//! The folder's frosted front: a pane with a tab on its left, showing the
//! newest photo blurred through it.

use crate::shape::ROUNDING;
use crownui::{
    ext::{DrawCommand, DrawList, ImageId, Paint, Path, Stroke},
    prelude::{Color, CornerRadii, Point, Rect, Size, Vector},
};

/// The tab's share of the pane's width, its height, and the run of the slope
/// down from it, in design points.
const TAB_SHARE: f32 = 0.44;
pub const TAB_HEIGHT: f32 = 15.0;
const TAB_SLOPE: f32 = 26.0;
const TOP_CORNER: f32 = 16.0;
pub const BOTTOM_CORNER: f32 = ROUNDING * 2.0 / 3.0;
const RIM: f32 = 1.2;
const SHADOW: (f32, f32) = (22.0, 10.0);

/// Whether the pane is drawn whole or only the tab standing above the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    Whole,
    Tab,
}

/// How the pane is tinted: light over light photos, dark over dark ones.
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
            Self::Light => Color::new(1.0, 1.0, 1.0, 0.4),
            Self::Dark => Color::new(0.07, 0.07, 0.08, 0.52),
        }
    }

    /// The pane with nothing showing through it.
    pub fn solid(self) -> Color {
        match self {
            Self::Light => Color::new(0.97, 0.97, 0.98, 0.97),
            Self::Dark => Color::new(0.11, 0.11, 0.12, 0.97),
        }
    }

    fn rim(self) -> Color {
        match self {
            Self::Light => Color::new(1.0, 1.0, 1.0, 0.75),
            Self::Dark => Color::new(1.0, 1.0, 1.0, 0.16),
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

/// The pane's outline in a box of `size`, `factor` design points to the point.
pub fn outline(size: Size, factor: f32, piece: Piece) -> Path {
    let (width, height) = (size.width, size.height);
    let (tab, slope, top, bottom) = (
        TAB_HEIGHT * factor,
        TAB_SLOPE * factor,
        TOP_CORNER * factor,
        BOTTOM_CORNER * factor,
    );
    let tab_end = width * TAB_SHARE;
    let shoulder = Path::new()
        .move_to(Point::new(0.0, top))
        .quad_to(Point::new(0.0, 0.0), Point::new(top, 0.0))
        .line_to(Point::new(tab_end - slope / 2.0, 0.0))
        .cubic_to(
            Point::new(tab_end, 0.0),
            Point::new(tab_end, tab),
            Point::new(tab_end + slope / 2.0, tab),
        );
    match piece {
        Piece::Tab => shoulder
            .line_to(Point::new(tab_end + slope / 2.0, tab + RIM))
            .line_to(Point::new(0.0, tab + RIM))
            .close(),
        Piece::Whole => shoulder
            .line_to(Point::new(width - top, tab))
            .quad_to(Point::new(width, tab), Point::new(width, tab + top))
            .line_to(Point::new(width, height - bottom))
            .quad_to(
                Point::new(width, height),
                Point::new(width - bottom, height),
            )
            .line_to(Point::new(bottom, height))
            .quad_to(Point::new(0.0, height), Point::new(0.0, height - bottom))
            .close(),
    }
}

/// The pane, frosted over `haze` (the newest photo, blurred) when there is one.
pub fn paint(
    list: &mut DrawList,
    size: Size,
    factor: f32,
    piece: Piece,
    tone: Tone,
    haze: Option<ImageId>,
) {
    let path = outline(size, factor, piece);
    if piece == Piece::Whole {
        let (blur, drop) = SHADOW;
        let body = Rect::from_xywh(
            0.0,
            TAB_HEIGHT * factor,
            size.width,
            size.height - TAB_HEIGHT * factor,
        );
        list.shadow(
            body,
            CornerRadii::uniform(BOTTOM_CORNER * factor),
            Color::new(0.0, 0.0, 0.0, 0.14),
            blur * factor,
            Vector::new(0.0, drop * factor),
        );
    }
    let fill = match haze {
        Some(haze) => {
            let whole = Rect::from_xywh(0.0, 0.0, size.width, size.height);
            list.push(DrawCommand::FillPath(
                path.clone(),
                Paint::image(haze, whole),
            ));
            tone.frost()
        }
        None => tone.solid(),
    };
    list.push(DrawCommand::FillPath(path.clone(), Paint::solid(fill)));
    if piece == Piece::Whole {
        list.push(DrawCommand::StrokePath(
            path,
            Stroke::new(RIM * factor),
            Paint::solid(tone.rim()),
        ));
    }
}

/// The pane with nothing showing through, laid over it while the folder is open.
pub fn paint_solid(list: &mut DrawList, size: Size, factor: f32, tone: Tone) {
    let path = outline(size, factor, Piece::Whole);
    list.push(DrawCommand::FillPath(path, Paint::solid(tone.solid())));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_sits_inside_the_whole_pane() {
        let size = Size::new(200.0, 150.0);
        let whole = outline(size, 1.0, Piece::Whole).bounds();
        let tab = outline(size, 1.0, Piece::Tab).bounds();
        assert!(tab.max_y() <= TAB_HEIGHT + RIM + 0.01);
        assert!(tab.max_x() < whole.max_x());
        assert!((whole.max_y() - 150.0).abs() < 0.01);
    }
}
