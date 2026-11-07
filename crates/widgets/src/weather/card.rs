use crownui::{
    ext::{Brush, GradientStop, Paint},
    prelude::{
        Color, CornerRadii, Cx, Memo, Point, Rect, Runtime, Signal, Size, Styled, View, WHITE,
        bind, canvas, hstack, icon, rgb, rgba, spacer, text, vstack, zstack,
    },
};

use super::{MISSING, degrees, glyph, reading};
use crate::{Env, Shape, art, scale::Scale, shape::ROUNDING};
use crownhome_services::weather::{Conditions, Forecast, Hour, Sky};

const DESIGN: (f32, f32) = (237.5, 241.0);
const HOURS: usize = 3;
const INK_SOFT: Color = rgba(0xFFFFFFD9);
const BUBBLE: Color = rgba(0xFFFFFF1F);
const LOCATION: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><path d="M3 11 22 2l-9 19-2-8z"/></svg>"#;

fn measure(size: Size) -> f32 {
    (size.height / DESIGN.1).min(size.width / DESIGN.0)
}

pub fn shape(size: Size) -> Shape {
    Shape::filling(size, ROUNDING * measure(size))
}

/// The top and bottom of the card's sky, darker at night.
fn tint(conditions: Option<Conditions>) -> [Color; 2] {
    let Some(conditions) = conditions else {
        return [rgb(0x5A6270), rgb(0x2B3038)];
    };
    match (conditions.sky(), conditions.daytime) {
        (Sky::Clear, true) => [rgb(0x4A9BEA), rgb(0x1E5CBF)],
        (Sky::PartlyCloudy, true) => [rgb(0x6E93BF), rgb(0x34557F)],
        (Sky::Clear | Sky::PartlyCloudy, false) => [rgb(0x2A3556), rgb(0x0E1426)],
        (Sky::Snow, _) => [rgb(0x8E9CB2), rgb(0x56657A)],
        (Sky::Thunder, _) => [rgb(0x3D4252), rgb(0x15181F)],
        (Sky::Rain, _) => [rgb(0x535E6C), rgb(0x242B34)],
        (Sky::Cloudy | Sky::Fog, _) => [rgb(0x5E6672), rgb(0x2B3038)],
    }
}

fn backdrop(forecast: Signal<Option<Forecast>>, scale: Memo<f32>) -> impl View {
    canvas(move |runtime, size, list| {
        let [top, bottom] = tint(forecast.with(runtime, |forecast| {
            forecast.as_ref().map(|forecast| forecast.now)
        }));
        let sky = Paint {
            brush: Brush::LinearGradient {
                start: Point::new(0.0, 0.0),
                end: Point::new(size.width * 0.3, size.height),
                stops: [(0.0, top), (1.0, bottom)]
                    .into_iter()
                    .map(|(offset, color)| GradientStop { offset, color })
                    .collect(),
            },
            blur: None,
        };
        list.fill(
            Rect::from_xywh(0.0, 0.0, size.width, size.height),
            CornerRadii::uniform(ROUNDING * scale.get(runtime)),
            sky,
        );
    })
    .absolute()
    .inset(0.0)
}

/// `7:00` as `07:00 AM`.
fn clock_label(hour: i8) -> String {
    let twelve_hour = match hour % 12 {
        0 => 12,
        hour => hour,
    };
    let half = if hour < 12 { "AM" } else { "PM" };
    format!("{twelve_hour:02}:00 {half}")
}

fn hour_column(
    cx: &mut Cx,
    forecast: Signal<Option<Forecast>>,
    index: usize,
    scale: Scale,
) -> impl View + use<> {
    let hour = move |runtime: &mut Runtime| -> Option<Hour> {
        forecast.with(runtime, |forecast| {
            forecast.as_ref()?.hours.get(index).copied()
        })
    };
    let glyph_edge = scale.length(cx, 30.0);
    let bubble = vstack(art::picture(
        cx,
        move |runtime| glyph(hour(runtime).map(|hour| hour.conditions)),
        glyph_edge,
    ))
    .w(scale.px(44.0))
    .h(scale.px(44.0))
    .rounded(scale.px(22.0))
    .bg(BUBBLE)
    .items_center()
    .justify_center();
    let time = text(bind(move |runtime| {
        hour(runtime).map_or_else(|| MISSING.to_owned(), |hour| clock_label(hour.hour))
    }))
    .text_size(scale.px(12.5))
    .text_color(INK_SOFT);
    let temperature = text(bind(move |runtime| {
        hour(runtime).map_or_else(
            || MISSING.to_owned(),
            |hour| format!("{}°", degrees(hour.temperature)),
        )
    }))
    .text_size(scale.px(17.0))
    .font_weight(600)
    .text_color(WHITE);
    vstack((bubble, time.mt(scale.px(3.0)), temperature))
        .items_center()
        .gap(scale.px(3.0))
}

pub fn view(cx: &mut Cx, name: &str, forecast: Signal<Option<Forecast>>, env: Env) -> impl View {
    let scale = Scale::new(cx, env.size, measure);
    let temperature = text(bind(reading(
        forecast,
        |forecast| format!("{}°", degrees(forecast.temperature)),
        "--°",
    )))
    .text_size(scale.px(50.0))
    .font_weight(500)
    .tracking(scale.px(-1.5))
    .text_color(WHITE);
    let place = hstack((
        text(name.to_owned())
            .text_size(scale.px(15.5))
            .truncate()
            .text_color(INK_SOFT)
            .shrink(1.0)
            .min_w(0.0),
        icon(LOCATION, INK_SOFT)
            .w(scale.px(11.0))
            .h(scale.px(11.0))
            .shrink(0.0),
    ))
    .items_center()
    .gap(scale.px(5.0));
    let condition = text(bind(reading(
        forecast,
        |forecast| forecast.now.description().to_owned(),
        "",
    )))
    .text_size(scale.px(15.5))
    .font_weight(500)
    .text_right()
    .line_clamp(2)
    .text_color(WHITE)
    .w(scale.px(84.0))
    .shrink(0.0)
    .mt(scale.px(7.0));
    let header = vstack((
        hstack((temperature, spacer(), condition))
            .w_full()
            .items_start(),
        place.mt(scale.px(-4.0)),
    ))
    .w_full();
    let hours: Vec<_> = (0..HOURS)
        .map(|index| hour_column(cx, forecast, index, scale))
        .collect();
    let content = vstack((header, spacer(), hstack(hours).w_full().justify_between()))
        .w_full()
        .h_full()
        .px(scale.px(20.0))
        .pt(scale.px(14.0))
        .pb(scale.px(16.0));
    zstack((backdrop(forecast, scale.factor()), content))
        .relative()
        .w_full()
        .h_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_read_on_a_twelve_hour_clock() {
        assert_eq!(clock_label(7), "07:00 AM");
        assert_eq!(clock_label(0), "12:00 AM");
        assert_eq!(clock_label(12), "12:00 PM");
        assert_eq!(clock_label(21), "09:00 PM");
    }
}
