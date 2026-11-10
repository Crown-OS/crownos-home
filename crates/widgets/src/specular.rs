//! The light on a widget's glass: a hairline all around its edge with a soft
//! glow just inside it, and a bright streak where the light strikes the
//! top-left corner, and more faintly the bottom-right, fading along the edges.

use std::f32::consts::FRAC_1_SQRT_2;

use crownui::{
    ext::{Brush, DrawList, GradientStop, Paint},
    prelude::{Color, CornerRadii, Point, Rect, Styled, Vector, View, canvas},
};

use crate::{Env, Shape, palette::Sheen};

const HAIRLINE: f32 = 1.0;
const GLOW_WIDTH: f32 = 8.0;
const GLOW_BLUR: f32 = 3.0;
const STREAK_WIDTH: f32 = 1.75;
const STREAK_GLOW_WIDTH: f32 = 10.0;
const STREAK_GLOW_BLUR: f32 = 4.0;
const STREAK_GLOW_STRENGTH: f32 = 0.5;
/// How far a streak runs from its corner, in corner radii, and how much of
/// that it stays at full strength.
const REACH: f32 = 2.6;
const FULL: f32 = 0.35;
const FAR_CORNER_STRENGTH: f32 = 0.6;

/// Light falling from `corner` along `inward`, gone `reach` points away.
fn corner_light(
    corner: Point,
    inward: Vector,
    reach: f32,
    color: Color,
    blur: Option<f32>,
) -> Paint {
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
        blur,
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
        CornerRadii::uniform((outline.radius - inset).max(0.0)),
        width,
        paint,
    );
}

/// A soft band along the outline, clipped to the glass, so it is brightest at
/// the edge and fades inwards.
fn glow_along(list: &mut DrawList, outline: Shape, width: f32, paint: Paint) {
    list.stroke(
        outline.bounds,
        CornerRadii::uniform(outline.radius),
        width,
        paint,
    );
}

fn paint_streaks(list: &mut DrawList, outline: Shape, color: Color) {
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
    for (corner, inward, color) in lights {
        let glow = color.with_alpha_factor(STREAK_GLOW_STRENGTH);
        glow_along(
            list,
            outline,
            STREAK_GLOW_WIDTH,
            corner_light(corner, inward, reach, glow, Some(STREAK_GLOW_BLUR)),
        );
        stroke_inside(
            list,
            outline,
            STREAK_WIDTH,
            corner_light(corner, inward, reach, color, None),
        );
    }
}

fn paint_sheen(list: &mut DrawList, outline: Shape, sheen: Sheen) {
    list.save();
    list.clip(outline.bounds, CornerRadii::uniform(outline.radius));
    glow_along(
        list,
        outline,
        GLOW_WIDTH,
        Paint {
            blur: Some(GLOW_BLUR),
            ..Paint::solid(sheen.glow)
        },
    );
    stroke_inside(list, outline, HAIRLINE, Paint::solid(sheen.rim));
    paint_streaks(list, outline, sheen.specular);
    list.restore();
}

/// The light on the widget of `kind`, following the outline it draws.
pub fn highlight(kind: String, env: Env) -> impl View {
    let palette = env.ambient.palette;
    canvas(move |runtime, size, list| {
        let sheen = palette.get(runtime).sheen;
        paint_sheen(list, crate::shape(&kind, size), sheen);
    })
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}
