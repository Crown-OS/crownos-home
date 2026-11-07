//! One occasion on a frosted pill: what it is, when, and how far off.

use crownui::prelude::{Cx, Runtime, Size, Styled, View, bind, hstack, show, spacer, text, vstack};
use jiff::Zoned;

use crate::{
    Built, Env, Shape,
    glass::glass,
    occasion::{Occasion, Schedule, days_between},
    scale::{Scale, ink},
};
use crownhome_board::WidgetSpec;

pub const KIND: &str = "event";

const DESIGN_PILL: f32 = 74.0;
const FILL: f32 = 0.8;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    detail: String,
    days: Option<String>,
}

fn row(schedule: &Schedule, now: &Zoned) -> Row {
    let today = now.date();
    let Some(day) = schedule.next(now) else {
        return Row {
            detail: "Over".to_owned(),
            days: None,
        };
    };
    match schedule.hours().filter(|_| day == today) {
        Some(hours) => Row {
            detail: hours,
            days: None,
        },
        None => Row {
            detail: format!("{} {}", day.strftime("%B"), day.day()),
            days: Some(days_left(days_between(today, day))),
        },
    }
}

fn days_left(days: i64) -> String {
    match days {
        0 => "Today".to_owned(),
        1 => "Tomorrow".to_owned(),
        days => format!("{days} days"),
    }
}

pub fn shape(size: Size) -> Shape {
    let height = size.height * FILL;
    Shape::band(size, height, height / 2.0)
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let occasion: Occasion = spec.decode()?;
    let schedule = occasion.schedule()?;
    let (now, palette) = (env.ambient.now, env.ambient.palette);
    let shown = cx.memo(move |runtime| now.with(runtime, |now| row(&schedule, now)));
    let read = move |pick: fn(&Row) -> String| {
        bind(move |runtime: &mut Runtime| shown.with(runtime, pick))
    };
    let scale = Scale::new(cx, env.size, |size| {
        (size.height * FILL / DESIGN_PILL).min(size.width / 230.0)
    });
    let title = text(occasion.title)
        .text_size(scale.px(21.0))
        .font_weight(510)
        .tracking(scale.px(-0.3))
        .truncate()
        .text_color(ink(palette, |palette| palette.ink));
    let detail = text(read(|row| row.detail.clone()))
        .text_size(scale.px(18.5))
        .tracking(scale.px(-0.4))
        .truncate()
        .text_color(ink(palette, |palette| palette.ink_secondary));
    let days = show(
        bind(move |runtime: &mut Runtime| shown.with(runtime, |row| row.days.is_some())),
        move || {
            text(read(|row| row.days.clone().unwrap_or_default()))
                .text_size(scale.px(19.5))
                .font_weight(450)
                .text_color(ink(palette, |palette| palette.ink_accent))
                .shrink(0.0)
        },
        || vstack(()),
    )
    .shrink(0.0);
    let pill = hstack((
        vstack((title, detail)).shrink(1.0).min_w(0.0),
        spacer(),
        days,
    ))
    .items_center()
    .gap(scale.px(6.0))
    .w_full()
    .h(bind(move |runtime| env.size.get(runtime).height * FILL))
    .px(scale.px(23.0))
    .rounded(bind(move |runtime| {
        env.size.get(runtime).height * FILL / 2.0
    }));
    Ok(vstack(glass(pill, env, |palette| palette.frost))
        .w_full()
        .h_full()
        .justify_center()
        .boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule(fields: &str) -> Schedule {
        let occasion: Occasion =
            toml::from_str(&format!("title = \"x\"\n{fields}")).expect("an occasion");
        occasion.schedule().expect("a schedule")
    }

    fn at(time: &str) -> Zoned {
        time.parse().expect("a time")
    }

    #[test]
    fn todays_meeting_shows_its_hours() {
        let meeting = schedule("weekly = \"sunday\"\nstart = \"17:00\"\nend = \"18:00\"");
        let shown = row(&meeting, &at("2026-05-24T08:15:30+02:00[Europe/Berlin]"));
        assert_eq!(shown.detail, "17:00 - 18:00");
        assert_eq!(shown.days, None);
    }

    #[test]
    fn a_birthday_counts_the_days_to_it() {
        let birthday = schedule("yearly = \"06-02\"");
        let shown = row(&birthday, &at("2026-05-24T08:15:30+02:00[Europe/Berlin]"));
        assert_eq!(shown.detail, "June 2");
        assert_eq!(shown.days.as_deref(), Some("9 days"));
    }

    #[test]
    fn a_meeting_that_has_ended_drops_to_next_week() {
        let meeting = schedule("weekly = \"sunday\"\nstart = \"17:00\"\nend = \"18:00\"");
        let shown = row(&meeting, &at("2026-05-24T19:00:00+02:00[Europe/Berlin]"));
        assert_eq!(shown.detail, "May 31");
        assert_eq!(shown.days.as_deref(), Some("7 days"));
    }
}
