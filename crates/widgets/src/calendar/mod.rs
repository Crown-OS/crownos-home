//! Today and what comes next from the user's Google calendars: the date and
//! its week on the left, the coming meetings on the right.

mod agenda;
mod day;
mod schedule;

use crownui::prelude::{Cx, Size, Styled, View, hstack};
use serde::{Deserialize, Serialize};

use crate::{Built, Env, Shape, face, scale::Scale, shape::ROUNDING};
use crownhome_board::WidgetSpec;
use crownhome_services::calendar::{self, Agenda};

pub const KIND: &str = "calendar";

const DESIGN: (f32, f32) = (512.5, 241.0);
const PADDING: f32 = 14.0;
/// The day page's corner, concentric with the card's.
const PAGE_ROUNDING: f32 = ROUNDING - PADDING;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {}

fn measure(size: Size) -> f32 {
    (size.height / DESIGN.1).min(size.width / DESIGN.0)
}

pub fn shape(size: Size) -> Shape {
    Shape::filling(size, ROUNDING * measure(size))
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let _: Settings = spec.decode()?;
    let agenda = cx.signal(None::<Agenda>);
    if env.is_live() {
        calendar::follow(cx, agenda);
    } else {
        agenda.set(cx, Some(schedule::sample(&jiff::Zoned::now())));
    }
    let scale = Scale::new(cx, env.size, measure);
    let now = env.ambient.now;
    let summary = cx.memo(move |runtime| {
        let now = now.get(runtime);
        agenda.with(runtime, |agenda| {
            schedule::Summary::of(agenda.as_ref(), &now)
        })
    });
    let card = hstack((
        day::view(summary, env, scale),
        agenda::view(summary, env, scale),
    ))
    .w_full()
    .h_full()
    .p(scale.px(PADDING))
    .gap(scale.px(18.0))
    .rounded(scale.px(ROUNDING))
    .overflow_hidden();
    Ok(face::under(card, env, scale.length(cx, ROUNDING)).boxed())
}
