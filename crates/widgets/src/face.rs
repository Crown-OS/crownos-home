//! The clock's face as a card: glass lit from the top-left, fading towards
//! the bottom-right.

use crownui::{
    ext::{Brush, DrawList, GradientStop, Paint},
    prelude::{CornerRadii, Memo, Point, Rect, Size, Styled, View, canvas, zstack},
};

use crate::{Env, palette::Dial, palette::MATERIAL};

const LIGHT_STOPS: [f32; 3] = [0.0, 0.55, 1.0];

/// The face filling `size`, its corners rounded to `corner`.
pub fn paint(list: &mut DrawList, size: Size, corner: f32, dial: &Dial) {
    let bounds = Rect::from_xywh(0.0, 0.0, size.width, size.height);
    let stops = dial
        .face
        .iter()
        .zip(LIGHT_STOPS)
        .map(|(color, offset)| GradientStop {
            offset,
            color: *color,
        })
        .collect();
    let light = Paint {
        brush: Brush::LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(size.width, size.height),
            stops,
        },
        blur: None,
    };
    list.fill(bounds, CornerRadii::uniform(corner), light);
}

/// `card` laid on the clock's face, both rounded to `corner`.
pub fn under(card: impl View + 'static, env: Env, corner: Memo<f32>) -> impl View {
    let palette = env.ambient.palette;
    let face = canvas(move |runtime, size, list| {
        paint(list, size, corner.get(runtime), &palette.get(runtime).dial);
    })
    .absolute()
    .inset(0.0);
    let stacked = zstack((face, card))
        .relative()
        .w_full()
        .h_full()
        .rounded(crownui::prelude::bind(move |runtime| corner.get(runtime)));
    if env.is_frosted() {
        stacked.material(MATERIAL)
    } else {
        stacked
    }
}
