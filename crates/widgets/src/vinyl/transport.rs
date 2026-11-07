use crownui::prelude::{Runtime, Signal, Styled, View, WHITE, hstack, icon, keyed, rgb, vstack};

use crate::{
    Env,
    music::{NEXT, PAUSE, PLAY, PREVIOUS, button},
    scale::Scale,
};
use crownhome_services::music::{Playback, Transport};

const INK: u32 = 0x141416;
const ROW_TOP: f32 = 690.0;
const ROW_HEIGHT: f32 = 200.0;
const GAP: f32 = 64.0;
const SKIP_EDGE: f32 = 116.0;
const STICKER: (f32, f32) = (236.0, 156.0);
const PAPER: f32 = 8.0;
const RULE_INSET: f32 = 11.0;
const RULE: f32 = 4.5;
/// Half the sticker's height, so its ends are round.
const CORNER: f32 = STICKER.1 / 2.0;
const GLYPH_EDGE: f32 = 86.0;

/// The play button, printed like a record-shop sticker: white paper, a black
/// label and a thin white rule inside it.
fn sticker(playback: Signal<Option<Playback>>, scale: Scale) -> impl View {
    let playing = move |runtime: &mut Runtime| {
        playback.with(runtime, |playback| {
            playback.as_ref().is_some_and(|playback| playback.playing)
        })
    };
    let glyph = keyed(playing, move |_, playing| {
        icon(if playing { PAUSE } else { PLAY }, WHITE)
            .w(scale.px(GLYPH_EDGE))
            .h(scale.px(GLYPH_EDGE))
    });
    let ruled = vstack(glyph)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .border(scale.px(RULE))
        .border_color(WHITE)
        .rounded(scale.px(CORNER - PAPER - RULE_INSET));
    let label = vstack(ruled)
        .w_full()
        .h_full()
        .p(scale.px(RULE_INSET))
        .rounded(scale.px(CORNER - PAPER))
        .bg(rgb(INK));
    vstack(label)
        .w(scale.px(STICKER.0))
        .h(scale.px(STICKER.1))
        .p(scale.px(PAPER))
        .rounded(scale.px(CORNER))
        .bg(WHITE)
}

/// Previous, play or pause, and next, in ink across the front of the sleeve.
pub fn view(playback: Signal<Option<Playback>>, env: Env, scale: Scale) -> impl View {
    let live = env.is_live();
    let skip = |svg: &'static str, label: &'static str, pressed: Transport| {
        let glyph = icon(svg, rgb(INK))
            .w(scale.px(SKIP_EDGE))
            .h(scale.px(SKIP_EDGE));
        button(glyph, label, playback, pressed, live)
    };
    hstack((
        skip(PREVIOUS, "Previous Track", Transport::Previous),
        button(
            sticker(playback, scale),
            "Play or Pause",
            playback,
            Transport::Play,
            live,
        ),
        skip(NEXT, "Next Track", Transport::Next),
    ))
    .absolute()
    .left(0.0)
    .right(0.0)
    .top(scale.px(ROW_TOP))
    .h(scale.px(ROW_HEIGHT))
    .items_center()
    .justify_center()
    .gap(scale.px(GAP))
}
