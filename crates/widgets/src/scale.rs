//! Lengths that follow a widget's frame.
//!
//! Each widget is drawn at a design size; `Scale` is how much bigger or
//! smaller its frame is than that, and every length it hands out tracks it.

use crownui::prelude::{Color, Cx, IntoProp, Memo, Size, bind};

use crate::palette::Palette;

#[derive(Clone, Copy)]
pub struct Scale(Memo<f32>);

impl Scale {
    pub fn new(cx: &mut Cx, size: Memo<Size>, measure: fn(Size) -> f32) -> Self {
        Self(cx.memo(move |runtime| measure(size.get(runtime)).max(0.0)))
    }

    pub fn factor(self) -> Memo<f32> {
        self.0
    }

    /// `length` design points as a value of its own, for a picture's edge.
    pub fn length(self, cx: &mut Cx, length: f32) -> Memo<f32> {
        let factor = self.0;
        cx.memo(move |runtime| factor.get(runtime) * length)
    }

    /// `length` design points, in the frame's.
    pub fn px(self, length: f32) -> impl IntoProp<f32> {
        let factor = self.0;
        bind(move |runtime| factor.get(runtime) * length)
    }
}

/// A palette colour that follows light and dark.
pub fn ink(
    palette: Memo<Palette>,
    pick: impl Fn(&Palette) -> Color + 'static,
) -> impl IntoProp<Color> {
    bind(move |runtime| pick(&palette.get(runtime)))
}
