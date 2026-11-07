//! An analog world clock on a glass squircle dial.

use std::f32::consts::{FRAC_PI_2, TAU};

use crownui::{
    ext::{DrawCommand, DrawList, Paint, Path, Stroke},
    prelude::{
        Color, CornerRadii, Cx, Memo, Point, Rect, Size, Styled, Vector, View, bind, canvas, text,
        vstack, zstack,
    },
};
use jiff::{Zoned, tz::TimeZone};
use serde::{Deserialize, Serialize};

use crate::{
    Built, Env, Shape, face,
    palette::{Dial, MATERIAL},
    scale::ink,
    shape::{ROUNDING, TILE},
};
use crownhome_board::WidgetSpec;

pub const KIND: &str = "clock";

const TICK_INSET: f32 = 0.04;
const MINUTE_TICK: (f32, f32) = (0.05, 0.011);
const HOUR_TICK: (f32, f32) = (0.095, 0.017);
const NUMERAL_RADIUS: f32 = 0.775;
const NUMERAL_SIZE: f32 = 0.18;
const HOUR_HAND: (f32, f32) = (0.58, 0.044);
const MINUTE_HAND: (f32, f32) = (0.95, 0.031);
const SECOND_HAND: (f32, f32, f32) = (0.26, 0.97, 0.016);
const PIVOT: (f32, f32) = (0.045, 0.021);
/// The corner as a fraction of the dial's half-edge, rounding like every other widget.
const CORNER: f32 = 2.0 * ROUNDING / TILE.width;
const PADDING: f32 = 0.12;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// An IANA name such as `Europe/Berlin`; the system's zone when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default = "seconds_by_default")]
    pub seconds: bool,
}

const fn seconds_by_default() -> bool {
    true
}

impl Settings {
    pub fn in_zone(timezone: Option<String>) -> Self {
        Self {
            timezone,
            seconds: seconds_by_default(),
        }
    }
}

fn zone(name: Option<&str>) -> TimeZone {
    name.map_or_else(TimeZone::system, |name| {
        TimeZone::get(name).unwrap_or_else(|e| {
            log::warn!("clock: {name}: {e}; using the system zone");
            TimeZone::system()
        })
    })
}

/// Hours, minutes and seconds in `zone`; seconds stay at zero without a
/// second hand, so the dial only changes once a minute.
fn reading(now: &Zoned, zone: &TimeZone, seconds: bool) -> (i8, i8, i8) {
    let local = now.with_time_zone(zone.clone());
    let second = if seconds { local.second() } else { 0 };
    (local.hour(), local.minute(), second)
}

/// The unit vector pointing at `turns` of a full turn, clockwise from twelve.
fn direction(turns: f32) -> Vector {
    let angle = turns * TAU - FRAC_PI_2;
    Vector::new(angle.cos(), angle.sin())
}

fn line(path: Path, center: Point, toward: Vector, from: f32, to: f32) -> Path {
    path.move_to(center + toward * from)
        .line_to(center + toward * to)
}

fn disc(list: &mut DrawList, center: Point, radius: f32, color: Color) {
    let bounds = Rect::from_xywh(
        center.x - radius,
        center.y - radius,
        2.0 * radius,
        2.0 * radius,
    );
    list.fill(bounds, CornerRadii::uniform(radius), Paint::solid(color));
}

/// Half the dial's edge, and the radius its ticks, numerals and hands reach within the padding.
fn reach(size: Size) -> (f32, f32) {
    let radius = size.width.min(size.height) / 2.0;
    (radius, radius * (1.0 - PADDING))
}

fn paint_face(list: &mut DrawList, size: Size, dial: &Dial) {
    let (edge, radius) = reach(size);
    let center = Point::new(size.width / 2.0, size.height / 2.0);
    face::paint(list, size, edge * CORNER, dial);
    let outer = radius * (1.0 - TICK_INSET);
    let (minutes, hours) = (0..60).fold((Path::new(), Path::new()), |(minutes, hours), minute| {
        let toward = direction(minute as f32 / 60.0);
        if minute % 5 == 0 {
            let hours = line(hours, center, toward, outer - radius * HOUR_TICK.0, outer);
            (minutes, hours)
        } else {
            let minutes = line(
                minutes,
                center,
                toward,
                outer - radius * MINUTE_TICK.0,
                outer,
            );
            (minutes, hours)
        }
    });
    list.push(DrawCommand::StrokePath(
        minutes,
        Stroke::new(radius * MINUTE_TICK.1),
        Paint::solid(dial.minute_tick),
    ));
    list.push(DrawCommand::StrokePath(
        hours,
        Stroke::new(radius * HOUR_TICK.1),
        Paint::solid(dial.hour_tick),
    ));
}

fn paint_hands(
    list: &mut DrawList,
    size: Size,
    (hour, minute, second): (i8, i8, i8),
    seconds: bool,
    dial: &Dial,
) {
    let (_, radius) = reach(size);
    let center = Point::new(size.width / 2.0, size.height / 2.0);
    let minutes = f32::from(minute) + f32::from(second) / 60.0;
    let hours = f32::from(hour % 12) + minutes / 60.0;
    let mut hand = |turns: f32, from: f32, (length, width): (f32, f32), color: Color| {
        let path = line(Path::new(), center, direction(turns), from, radius * length);
        list.push(DrawCommand::StrokePath(
            path,
            Stroke::rounded(radius * width),
            Paint::solid(color),
        ));
    };
    hand(hours / 12.0, 0.0, HOUR_HAND, dial.hands);
    hand(minutes / 60.0, 0.0, MINUTE_HAND, dial.hands);
    if seconds {
        let (tail, length, width) = SECOND_HAND;
        hand(
            f32::from(second) / 60.0,
            -radius * tail,
            (length, width),
            dial.seconds,
        );
    }
    disc(list, center, radius * PIVOT.0, dial.seconds);
    disc(
        list,
        center,
        radius * PIVOT.1,
        Color::new(1.0, 1.0, 1.0, 1.0),
    );
}

fn numeral(hour: u8, edge: Memo<f32>, env: Env) -> impl View {
    let toward = direction(f32::from(hour) / 12.0);
    let radius = move |runtime: &mut crownui::prelude::Runtime| edge.get(runtime) * (1.0 - PADDING);
    let corner = move |runtime: &mut crownui::prelude::Runtime, axis: f32| {
        let radius = radius(runtime);
        edge.get(runtime) + axis * radius * NUMERAL_RADIUS - radius * NUMERAL_SIZE
    };
    vstack(
        text(hour.to_string())
            .text_size(bind(move |runtime| radius(runtime) * NUMERAL_SIZE))
            .font_weight(640)
            .text_color(ink(env.ambient.palette, |palette| palette.dial.numerals)),
    )
    .absolute()
    .left(bind(move |runtime| corner(runtime, toward.x)))
    .top(bind(move |runtime| corner(runtime, toward.y)))
    .w(bind(move |runtime| 2.0 * radius(runtime) * NUMERAL_SIZE))
    .h(bind(move |runtime| 2.0 * radius(runtime) * NUMERAL_SIZE))
    .items_center()
    .justify_center()
}

fn dial_radius(size: Size) -> f32 {
    (size.width.min(size.height) / 2.0 - 0.5).floor().max(1.0)
}

pub fn shape(size: Size) -> Shape {
    let radius = dial_radius(size);
    Shape::centred_square(size, 2.0 * radius, radius * CORNER)
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let zone = zone(settings.timezone.as_deref());
    let seconds = settings.seconds;
    let (now, palette) = (env.ambient.now, env.ambient.palette);
    let radius = cx.memo(move |runtime| dial_radius(env.size.get(runtime)));
    let time = cx.memo(move |runtime| now.with(runtime, |now| reading(now, &zone, seconds)));
    let face =
        canvas(move |runtime, size, list| paint_face(list, size, &palette.get(runtime).dial))
            .absolute()
            .inset(0.0);
    let hands = canvas(move |runtime, size, list| {
        paint_hands(
            list,
            size,
            time.get(runtime),
            seconds,
            &palette.get(runtime).dial,
        );
    })
    .absolute()
    .inset(0.0);
    let numerals: Vec<_> = (1..=12).map(|hour| numeral(hour, radius, env)).collect();
    let dial = zstack((face, numerals, hands))
        .relative()
        .w(bind(move |runtime| 2.0 * radius.get(runtime)))
        .h(bind(move |runtime| 2.0 * radius.get(runtime)))
        .rounded(bind(move |runtime| radius.get(runtime) * CORNER));
    let dial = if env.is_live() {
        dial.material(MATERIAL)
    } else {
        dial
    };
    Ok(vstack(dial)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_points_up_and_three_points_right() {
        let up = direction(0.0);
        assert!(up.x.abs() < 1e-6 && (up.y + 1.0).abs() < 1e-6);
        let right = direction(0.25);
        assert!((right.x - 1.0).abs() < 1e-6 && right.y.abs() < 1e-6);
    }

    #[test]
    fn the_hands_read_the_clocks_own_zone() {
        let utc_noon: Zoned = "2026-05-24T12:00:07+00:00[UTC]".parse().expect("a time");
        let tokyo = zone(Some("Asia/Tokyo"));
        assert_eq!(reading(&utc_noon, &tokyo, true), (21, 0, 7));
        assert_eq!(reading(&utc_noon, &tokyo, false), (21, 0, 0));
    }
}
