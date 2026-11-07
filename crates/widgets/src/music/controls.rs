use std::time::{Duration, Instant};

use crownui::prelude::{
    BLACK, CursorIcon, Listeners, Memo, Role, Semantic, Signal, Styled, View, WHITE, bind, hstack,
    icon, keyed, text, vstack,
};

use super::clock;
use crate::{
    Env,
    scale::{Scale, ink},
};
use crownhome_services::music::{self, Playback, Transport};

pub(crate) const PREVIOUS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><path d="M11.5 6.5v11L3.5 12zM20.5 6.5v11l-8-5.5z"/></svg>"#;
pub(crate) const NEXT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><path d="M12.5 6.5v11l8-5.5zM3.5 6.5v11l8-5.5z"/></svg>"#;
pub(crate) const PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><path d="M8 5.5v13l10.5-6.5z"/></svg>"#;
pub(crate) const PAUSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="black"><rect x="6.5" y="5" width="3.8" height="14" rx="1.2"/><rect x="13.7" y="5" width="3.8" height="14" rx="1.2"/></svg>"#;
const SKIP_EDGE: f32 = 22.0;
const PLAY_EDGE: f32 = 42.0;
const TIME_SIZE: f32 = 15.5;
/// How long a player takes to report the track a skip moved it to.
const PLAYER_CATCH_UP: Duration = Duration::from_millis(250);

/// What `pressed` asks of the player, given whether it is playing now.
fn command(pressed: Transport, playing: bool) -> Transport {
    match pressed {
        Transport::Play | Transport::Pause if playing => Transport::Pause,
        Transport::Play | Transport::Pause => Transport::Play,
        skip => skip,
    }
}

/// Sends `pressed` to wherever `playback` was read from, shows a play/pause
/// straight away, and reads the player again once it has acted instead of
/// waiting for the next poll.
fn press(cx: &mut crownui::prelude::Cx, playback: Signal<Option<Playback>>, pressed: Transport) {
    let Some((playing, source)) = playback.with(cx, |playback| {
        playback
            .as_ref()
            .map(|playback| (playback.playing, playback.source))
    }) else {
        return;
    };
    let sent = command(pressed, playing);
    if matches!(sent, Transport::Play | Transport::Pause) {
        playback.update(cx, |playback| {
            if let Some(playback) = playback {
                let now = Instant::now();
                playback.progress = playback.position(now);
                playback.read_at = now;
                playback.playing = sent == Transport::Play;
            }
        });
    }
    cx.spawn(
        async move {
            music::send(sent, source).await;
            tokio::time::sleep(PLAYER_CATCH_UP).await;
            music::now_playing().await
        },
        move |cx, fresh| playback.set(cx, fresh),
    );
}

pub(crate) fn button(
    content: impl View,
    label: &'static str,
    playback: Signal<Option<Playback>>,
    pressed: Transport,
    live: bool,
) -> impl View {
    vstack(content)
        .items_center()
        .justify_center()
        .cursor(CursorIcon::Pointer)
        .role(Role::Button)
        .label(label)
        .hover(|style| style.scale(1.08))
        .on_click(move |cx, event| {
            event.stop_propagation();
            if live {
                press(cx, playback, pressed);
            }
        })
}

pub fn view(
    playback: Signal<Option<Playback>>,
    position: Memo<Option<(Duration, Duration)>>,
    env: Env,
    scale: Scale,
) -> impl View {
    let palette = env.ambient.palette;
    let live = env.is_live();
    let skip = |svg: &'static str| {
        icon(svg, ink(palette, |palette| palette.ink_secondary))
            .w(scale.px(SKIP_EDGE))
            .h(scale.px(SKIP_EDGE))
    };
    let playing = move |runtime: &mut crownui::prelude::Runtime| {
        playback.with(runtime, |playback| {
            playback.as_ref().is_some_and(|playback| playback.playing)
        })
    };
    let toggle = keyed(playing, move |_, playing| {
        icon(
            if playing { PAUSE } else { PLAY },
            ink(palette, |palette| if palette.dark { BLACK } else { WHITE }),
        )
        .w(scale.px(PLAY_EDGE * 0.5))
        .h(scale.px(PLAY_EDGE * 0.5))
        .boxed()
    })
    .w(scale.px(PLAY_EDGE))
    .h(scale.px(PLAY_EDGE))
    .rounded(scale.px(PLAY_EDGE / 2.0))
    .bg(ink(palette, |palette| palette.ink))
    .items_center()
    .justify_center();
    let elapsed = text(bind(move |runtime| {
        position
            .get(runtime)
            .map_or_else(|| "0:00".to_owned(), |(at, _)| clock(at))
    }))
    .text_size(scale.px(TIME_SIZE))
    .font_weight(600)
    .text_color(ink(palette, |palette| palette.ink));
    let length = text(bind(move |runtime| {
        position.get(runtime).map_or_else(
            || " / 0:00".to_owned(),
            |(_, length)| format!(" / {}", clock(length)),
        )
    }))
    .text_size(scale.px(TIME_SIZE))
    .text_color(ink(palette, |palette| palette.ink_secondary));
    let buttons = hstack((
        button(
            skip(PREVIOUS),
            "Previous Track",
            playback,
            Transport::Previous,
            live,
        ),
        button(toggle, "Play or Pause", playback, Transport::Play, live),
        button(skip(NEXT), "Next Track", playback, Transport::Next, live),
    ))
    .w_full()
    .items_center()
    .justify_center()
    .gap(scale.px(7.0));
    let time = vstack(hstack((elapsed, length)).items_baseline())
        .absolute()
        .right(0.0)
        .top(0.0)
        .bottom(0.0)
        .justify_center();
    hstack((buttons, time)).relative().w_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_button_pauses_what_plays_and_plays_what_is_paused() {
        assert_eq!(command(Transport::Play, true), Transport::Pause);
        assert_eq!(command(Transport::Play, false), Transport::Play);
        assert_eq!(command(Transport::Next, true), Transport::Next);
    }
}
