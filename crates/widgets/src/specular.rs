//! The lit edge of glass: light catches the rim around the top-left corner
//! and, more faintly, the bottom-right, fading out along the edges and
//! glowing a little into the glass.

use std::f32::consts::FRAC_1_SQRT_2;

use crownui::{
    ext::{Brush, DrawList, GradientStop, Paint},
    prelude::{Color, CornerRadii, Point, Rect, Styled, Vector, View, canvas},
};

use crate::{Env, Shape};

const WIDTH: f32 = 2.0;
const GLOW_WIDTH: f32 = 6.0;
const GLOW_BLUR: f32 = 4.0;
const GLOW_STRENGTH: f32 = 0.5;
/// How far the light runs from a lit corner, in corner radii, and how much
/// of that it stays at full strength.
const REACH: f32 = 2.6;
const FULL: f32 = 0.35;
const FAR_CORNER_STRENGTH: f32 = 0.6;

/// Light falling from `corner` along `inward`, gone `reach` points away.
fn corner_light(corner: Point, inward: Vector, reach: f32, color: Color) -> Paint {
    let stops = [
        (0.0, color),
        (FULL, color),
        (1.0, color.with_alpha_factor(0.0)),
    ]
    .into_iter()
    .map(|(offset, color)| GradientStop { offset, color })
    .collect();
    Paint {
        brush: Brush::LinearGradient {
            start: corner,
            end: corner + inward * reach,
            stops,
        },
        blur: None,
    }
}

fn stroke_inside(list: &mut DrawList, outline: Shape, width: f32, paint: Paint) {
    let inset = width / 2.0;
    let bounds = outline.bounds;
    let rim = Rect::from_xywh(
        bounds.min_x() + inset,
        bounds.min_y() + inset,
        bounds.size.width - width,
        bounds.size.height - width,
    );
    list.stroke(
        rim,
        CornerRadii::uniform(outline.radius - inset),
        width,
        paint,
    );
}

fn paint_highlight(list: &mut DrawList, outline: Shape, color: Color) {
    let bounds = outline.bounds;
    let reach = outline.radius * REACH;
    let diagonal = Vector::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2);
    let lights = [
        (bounds.origin, diagonal, color),
        (
            Point::new(bounds.max_x(), bounds.max_y()),
            -diagonal,
            color.with_alpha_factor(FAR_CORNER_STRENGTH),
        ),
    ];
    list.save();
    list.clip(bounds, CornerRadii::uniform(outline.radius));
    for (corner, inward, color) in lights {
        let glow = corner_light(
            corner,
            inward,
            reach,
            color.with_alpha_factor(GLOW_STRENGTH),
        );
        stroke_inside(
            list,
            outline,
            GLOW_WIDTH,
            Paint {
                blur: Some(GLOW_BLUR),
                ..glow
            },
        );
        stroke_inside(
            list,
            outline,
            WIDTH,
            corner_light(corner, inward, reach, color),
        );
    }
    list.restore();
}

/// The highlight over the widget of `kind`, following the outline it draws.
pub fn highlight(kind: String, env: Env) -> impl View {
    let palette = env.ambient.palette;
    canvas(move |runtime, size, list| {
        let color = palette.get(runtime).specular;
        paint_highlight(list, crate::shape(&kind, size), color);
    })
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}
