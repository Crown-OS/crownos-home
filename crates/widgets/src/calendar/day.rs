use crownui::prelude::{
    Color, Memo, Runtime, Styled, View, bind, hstack, rgb, spacer, text, vstack,
};
use jiff::civil::Date;

use super::schedule::Summary;
use crate::{
    Env,
    scale::{Scale, ink},
};

const TODAY_INK: Color = rgb(0xF0453A);
const DAY_CIRCLE: f32 = 30.0;

fn header(summary: Memo<Summary>, env: Env, scale: Scale) -> impl View {
    let palette = env.ambient.palette;
    let read = move |pick: fn(&Summary) -> String| {
        bind(move |runtime: &mut Runtime| summary.with(runtime, pick))
    };
    let number = text(read(|summary| summary.today.day().to_string()))
        .text_size(scale.px(56.0))
        .tracking(scale.px(-2.0))
        .text_color(TODAY_INK);
    let weekday = text(read(|summary| summary.today.strftime("%A").to_string()))
        .text_size(scale.px(19.0))
        .font_weight(560)
        .text_color(ink(palette, |palette| palette.ink));
    let month = text(read(|summary| {
        let count = match summary.events_today {
            1 => "1 Event".to_owned(),
            count => format!("{count} Events"),
        };
        format!("{} · {count}", summary.today.strftime("%B"))
    }))
    .text_size(scale.px(15.0))
    .truncate()
    .text_color(ink(palette, |palette| palette.ink_secondary));
    hstack((
        number.shrink(0.0),
        vstack((weekday, month)).shrink(1.0).min_w(0.0),
    ))
    .items_center()
    .gap(scale.px(12.0))
}

fn week_day(summary: Memo<Summary>, slot: usize, env: Env, scale: Scale) -> impl View {
    let palette = env.ambient.palette;
    let date = move |runtime: &mut Runtime| summary.with(runtime, |summary| summary.week()[slot]);
    let is_today = move |runtime: &mut Runtime| {
        summary.with(runtime, |summary| summary.week()[slot] == summary.today)
    };
    let letter = |date: Date| {
        date.strftime("%a")
            .to_string()
            .chars()
            .next()
            .unwrap_or(' ')
    };
    let number = vstack(
        text(bind(move |runtime| date(runtime).day().to_string()))
            .text_size(scale.px(16.5))
            .font_weight(500)
            .text_color(bind(move |runtime| {
                if is_today(runtime) {
                    TODAY_INK
                } else {
                    palette.get(runtime).ink
                }
            })),
    )
    .w(scale.px(DAY_CIRCLE))
    .h(scale.px(DAY_CIRCLE))
    .rounded(scale.px(DAY_CIRCLE / 2.0))
    .items_center()
    .justify_center()
    .bg(bind(move |runtime| {
        if is_today(runtime) {
            TODAY_INK.with_alpha_factor(0.14)
        } else {
            TODAY_INK.with_alpha_factor(0.0)
        }
    }));
    let weekday = text(bind(move |runtime| letter(date(runtime)).to_string()))
        .text_size(scale.px(12.0))
        .text_color(ink(palette, |palette| palette.ink_secondary));
    vstack((number, weekday)).items_center().gap(scale.px(4.0))
}

pub fn view(summary: Memo<Summary>, env: Env, scale: Scale) -> impl View {
    let palette = env.ambient.palette;
    let week: Vec<_> = (0..7)
        .map(|slot| week_day(summary, slot, env, scale))
        .collect();
    let page = vstack((
        header(summary, env, scale),
        spacer(),
        hstack(week).w_full().justify_between(),
    ))
    .w_full()
    .grow(1.0)
    .px(scale.px(14.0))
    .pt(scale.px(8.0))
    .pb(scale.px(12.0))
    .rounded(scale.px(super::PAGE_ROUNDING))
    .bg(ink(palette, |palette| palette.ink.with_alpha_factor(0.05)));
    let note = text(bind(move |runtime| {
        summary.with(runtime, |summary| {
            summary
                .all_day
                .as_ref()
                .map_or_else(String::new, |title| format!("• {title}"))
        })
    }))
    .text_size(scale.px(14.0))
    .truncate()
    .text_color(ink(palette, |palette| palette.ink))
    .px(scale.px(6.0));
    vstack((page, note))
        .w(scale.px(250.0))
        .h_full()
        .shrink(0.0)
        .gap(scale.px(8.0))
}
