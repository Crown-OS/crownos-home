//! Days left until one date, on a card of its own colour.

use crownui::{
    ext::{Brush, GradientStop, Paint},
    prelude::{
        Color, CornerRadii, Cx, Memo, Point, Rect, Runtime, Size, Styled, View, bind, canvas,
        hstack, rgb, rgba, spacer, text, vstack, zstack,
    },
};
use jiff::{Zoned, civil::Date};
use serde::{Deserialize, Serialize};

use crate::{
    Built, Env, Shape,
    occasion::{Occasion, Schedule, days_between},
    scale::Scale,
};
use crownhome_board::WidgetSpec;
use crownhome_util::color::parse_hex;

pub const KIND: &str = "countdown";

const DESIGN: (f32, f32) = (237.5, 241.0);
const ROUNDING: f32 = 30.0;
const COUNT: f32 = 90.0;
/// Three digits at the full size run past the card's edge.
const LONG_COUNT: f32 = 70.0;
const INK: Color = rgb(0xFFFFFF);
const UNIT_INK: Color = rgba(0xFFFFFFEB);
const RIM: Color = rgba(0xFFFFFF66);
const DEFAULT_COLOR: Color = rgb(0x7B6EFD);
/// How far the card's light leans towards blue at the top left and violet at
/// the bottom right.
const SHADE: f32 = 0.12;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yearly: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekly: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl Settings {
    fn occasion(&self) -> Occasion {
        Occasion {
            title: self.title.clone(),
            date: self.date.clone(),
            yearly: self.yearly.clone(),
            weekly: self.weekly.clone(),
            ..Occasion::default()
        }
    }

    fn color(&self) -> Color {
        let Some(text) = &self.color else {
            return DEFAULT_COLOR;
        };
        parse_hex(text).unwrap_or_else(|| {
            log::warn!("countdown: `{text}` is not a colour like #7B6EFD");
            DEFAULT_COLOR
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Reading {
    count: String,
    date: String,
    weekday: String,
}

fn reading(schedule: &Schedule, today: Date) -> Reading {
    match schedule.on_or_after(today) {
        Some(day) => Reading {
            count: days_between(today, day).to_string(),
            date: format!("{}/{}/{}", day.year(), day.month(), day.day()),
            weekday: day.strftime("%A").to_string(),
        },
        None => Reading {
            count: "–".to_owned(),
            date: String::new(),
            weekday: String::new(),
        },
    }
}

fn card(color: Color, scale: Memo<f32>) -> impl View {
    canvas(move |runtime, size, list| {
        let radius = ROUNDING * scale.get(runtime);
        let bounds = Rect::from_xywh(0.0, 0.0, size.width, size.height);
        let stops = [
            (0.0, color.lerp(rgb(0x8C9CF0), SHADE)),
            (0.5, color),
            (1.0, color.lerp(rgb(0x7050FF), SHADE)),
        ]
        .into_iter()
        .map(|(offset, color)| GradientStop { offset, color })
        .collect();
        let light = Paint {
            brush: Brush::LinearGradient {
                start: Point::new(0.0, 0.0),
                end: Point::new(size.width, size.height),
                stops,
            },
            blur: None,
        };
        list.fill(bounds, CornerRadii::uniform(radius), light);
        let ring = Rect::from_xywh(0.5, 0.5, size.width - 1.0, size.height - 1.0);
        list.stroke(
            ring,
            CornerRadii::uniform(radius - 0.5),
            1.25,
            Paint::solid(RIM),
        );
    })
    .absolute()
    .inset(0.0)
}

pub fn shape(size: Size) -> Shape {
    let scale = (size.height / DESIGN.1).min(size.width / DESIGN.0);
    Shape::filling(size, ROUNDING * scale)
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let schedule = settings.occasion().schedule()?;
    let now = env.ambient.now;
    let today = cx.memo(move |runtime| now.with(runtime, Zoned::date));
    let shown = cx.memo(move |runtime| reading(&schedule, today.get(runtime)));
    let read = move |pick: fn(&Reading) -> String| {
        bind(move |runtime: &mut Runtime| shown.with(runtime, pick))
    };
    let scale = Scale::new(cx, env.size, |size| {
        (size.height / DESIGN.1).min(size.width / DESIGN.0)
    });
    let line = |content: String, size: f32| {
        text(content)
            .text_size(scale.px(size))
            .text_color(INK)
            .truncate()
    };
    let count = hstack((
        text(read(|reading| reading.count.clone()))
            .text_size(bind(move |runtime: &mut Runtime| {
                let long = shown.with(runtime, |reading| reading.count.len() > 2);
                scale.factor().get(runtime) * if long { LONG_COUNT } else { COUNT }
            }))
            .tracking(scale.px(-1.5))
            .text_color(INK),
        text("Days")
            .text_size(scale.px(17.5))
            .font_weight(420)
            .text_color(UNIT_INK)
            .mb(scale.px(16.0)),
    ))
    .items_end()
    .gap(scale.px(6.0));
    let column = vstack((
        line(settings.title.clone(), 21.5),
        count.mt(scale.px(-6.0)),
        spacer(),
        text(read(|reading| reading.date.clone()))
            .text_size(scale.px(21.0))
            .text_color(INK),
        text(read(|reading| reading.weekday.clone()))
            .text_size(scale.px(20.0))
            .text_color(INK),
    ))
    .w_full()
    .h_full()
    .px(scale.px(27.0))
    .pt(scale.px(19.0))
    .pb(scale.px(18.0));
    Ok(zstack((card(settings.color(), scale.factor()), column))
        .relative()
        .w_full()
        .h_full()
        .boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(fields: &str) -> Settings {
        toml::from_str(&format!("title = \"Till Birthday\"\n{fields}")).expect("settings")
    }

    #[test]
    fn the_reference_card_reads_86_days_to_a_tuesday() {
        let schedule = settings("yearly = \"08-18\"")
            .occasion()
            .schedule()
            .expect("a schedule");
        let shown = reading(&schedule, jiff::civil::date(2026, 5, 24));
        assert_eq!(shown.count, "86");
        assert_eq!(shown.date, "2026/8/18");
        assert_eq!(shown.weekday, "Tuesday");
    }

    #[test]
    fn a_colour_that_does_not_parse_falls_back() {
        assert_eq!(
            settings("yearly = \"01-01\"\ncolor = \"violet\"").color(),
            DEFAULT_COLOR
        );
    }
}
