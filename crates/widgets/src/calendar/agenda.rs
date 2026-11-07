use crownui::prelude::{
    Color, CursorIcon, Listeners, Memo, Role, Semantic, Styled, View, hstack, keyed, rgb, spacer,
    text, vstack,
};

use super::schedule::{Coming, Summary, Upcoming};
use crate::{
    Env,
    scale::{Scale, ink},
};
use crownhome_services::launch;
use crownhome_util::color::parse_hex;

const BAR_WIDTH: f32 = 4.0;
const MEET_INK: Color = rgb(0x4458D8);
const FALLBACK_BAR: Color = rgb(0x8E8E96);

fn meet_button(url: String, live: bool, scale: Scale) -> impl View {
    text("Go to Meet")
        .text_size(scale.px(14.5))
        .font_weight(600)
        .text_color(MEET_INK)
        .px(scale.px(11.0))
        .py(scale.px(5.0))
        .rounded(scale.px(10.0))
        .bg(MEET_INK.with_alpha_factor(0.12))
        .shrink(0.0)
        .cursor(CursorIcon::Pointer)
        .role(Role::Button)
        .label("Go to Meet")
        .on_click(move |_, event| {
            event.stop_propagation();
            if live {
                launch::run(vec!["xdg-open".to_owned(), url.clone()]);
            }
        })
}

fn meeting(meeting: Upcoming, env: Env, scale: Scale) -> impl View {
    let palette = env.ambient.palette;
    let bar_color = meeting
        .color
        .as_deref()
        .and_then(parse_hex)
        .unwrap_or(FALLBACK_BAR);
    let title = text(meeting.title)
        .text_size(scale.px(15.5))
        .line_clamp(2)
        .text_color(ink(palette, |palette| palette.ink));
    let detail = text(meeting.detail)
        .text_size(scale.px(13.5))
        .truncate()
        .text_color(ink(palette, |palette| palette.ink_secondary))
        .shrink(1.0)
        .min_w(0.0);
    let meet = match meeting.meeting_url {
        Some(url) => meet_button(url, env.is_live(), scale).boxed(),
        None => vstack(()).boxed(),
    };
    hstack((
        vstack(())
            .w(scale.px(BAR_WIDTH))
            .h_full()
            .rounded(scale.px(BAR_WIDTH / 2.0))
            .bg(bar_color)
            .shrink(0.0),
        vstack((
            title,
            hstack((detail, spacer(), meet))
                .w_full()
                .items_center()
                .gap(scale.px(8.0)),
        ))
        .gap(scale.px(6.0))
        .grow(1.0)
        .shrink(1.0)
        .min_w(0.0),
    ))
    .w_full()
    .gap(scale.px(12.0))
}

fn notice(words: &'static str, env: Env, scale: Scale) -> impl View {
    vstack(
        text(words)
            .text_size(scale.px(15.0))
            .text_center()
            .text_color(ink(env.ambient.palette, |palette| palette.ink_secondary)),
    )
    .w_full()
    .h_full()
    .items_center()
    .justify_center()
}

pub fn view(summary: Memo<Summary>, env: Env, scale: Scale) -> impl View {
    keyed(
        move |runtime| summary.with(runtime, |summary| summary.coming.clone()),
        move |_, coming| match coming {
            Coming::Loading => vstack(()).boxed(),
            Coming::SignedOut => {
                notice("Connect Google in Settings → Accounts", env, scale).boxed()
            }
            Coming::Events(meetings) if meetings.is_empty() => {
                notice("No more events this week", env, scale).boxed()
            }
            Coming::Events(meetings) => {
                let rows: Vec<_> = meetings
                    .into_iter()
                    .map(|upcoming| meeting(upcoming, env, scale).boxed())
                    .collect();
                vstack(rows)
                    .w_full()
                    .h_full()
                    .justify_center()
                    .gap(scale.px(18.0))
                    .boxed()
            }
        },
    )
    .grow(1.0)
    .shrink(1.0)
    .min_w(0.0)
    .h_full()
}
