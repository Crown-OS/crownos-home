use crownui::prelude::{Cx, Signal, Size, Styled, View, bind, hstack, spacer, text, vstack};

use super::{MISSING, degrees, glyph, reading};
use crate::{
    Env, Shape, art,
    glass::glass,
    scale::{Scale, ink},
};
use crownhome_services::weather::Forecast;

const DESIGN_HEIGHT: f32 = 88.0;
const DESIGN_WIDTH: f32 = 250.0;
const FILL: f32 = 0.88;
const ROUNDING: f32 = 28.0;

fn measure(size: Size) -> f32 {
    (size.height * FILL / DESIGN_HEIGHT).min(size.width / DESIGN_WIDTH)
}

pub fn shape(size: Size) -> Shape {
    Shape::band(size, size.height * FILL, ROUNDING * measure(size))
}

pub fn view(cx: &mut Cx, name: &str, forecast: Signal<Option<Forecast>>, env: Env) -> impl View {
    let palette = env.ambient.palette;
    let scale = Scale::new(cx, env.size, measure);
    let glyph_edge = scale.length(cx, 50.0);
    let temperature = text(bind(reading(
        forecast,
        |forecast| degrees(forecast.temperature),
        MISSING,
    )))
    .text_size(scale.px(58.0))
    .font_weight(380)
    .tracking(scale.px(-2.5))
    .text_color(ink(palette, |palette| palette.ink));
    let degree = text("°")
        .text_size(scale.px(34.0))
        .text_color(ink(palette, |palette| palette.ink))
        .self_start()
        .mt(scale.px(8.0));
    let name = text(name.to_owned())
        .text_size(scale.px(20.0))
        .font_weight(520)
        .truncate()
        .text_color(ink(palette, |palette| palette.ink));
    let range = text(bind(reading(
        forecast,
        |forecast| format!("{}°/{}°", degrees(forecast.high), degrees(forecast.low)),
        "--/--",
    )))
    .text_size(scale.px(17.0))
    .tracking(scale.px(-0.6))
    .truncate()
    .text_color(ink(palette, |palette| palette.ink_secondary));
    let conditions = move |runtime: &mut crownui::prelude::Runtime| {
        glyph(forecast.with(runtime, |forecast| {
            forecast.as_ref().map(|forecast| forecast.now)
        }))
    };
    let pill = hstack((
        temperature,
        degree,
        vstack((name, range))
            .ml(scale.px(10.0))
            .gap(scale.px(1.0))
            .shrink(1.0)
            .min_w(0.0),
        spacer(),
        art::picture(cx, conditions, glyph_edge),
    ))
    .items_center()
    .w_full()
    .h(bind(move |runtime| env.size.get(runtime).height * FILL))
    .pl(scale.px(17.5))
    .pr(scale.px(14.0))
    .rounded(scale.px(ROUNDING))
    .overflow_hidden();
    vstack(glass(pill, env, |palette| palette.frost))
        .w_full()
        .h_full()
        .justify_center()
}
