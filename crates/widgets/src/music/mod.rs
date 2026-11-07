//! What Spotify is playing: the cover, the lines being sung, how far along it
//! is, and buttons to skip and pause.

mod controls;
mod cover;
mod lines;

use std::time::{Duration, Instant};

use crownui::{
    ext::Paint,
    prelude::{CornerRadii, Cx, Memo, Rect, Size, Styled, View, canvas, hstack, spacer, vstack},
};
use serde::{Deserialize, Serialize};

use crate::{Built, Env, Shape, glass::glass, scale::Scale};
use crownhome_board::WidgetSpec;
use crownhome_services::music::{self, Playback, Source, Track};

pub const KIND: &str = "music";

const DESIGN: (f32, f32) = (237.5, 241.0);
const ROUNDING: f32 = 30.0;
const BAR_HEIGHT: f32 = 5.0;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {}

fn measure(size: Size) -> f32 {
    (size.height / DESIGN.1).min(size.width / DESIGN.0)
}

pub fn shape(size: Size) -> Shape {
    Shape::filling(size, ROUNDING * measure(size))
}

fn sample() -> Playback {
    Playback {
        track: Track {
            title: "Blessing in Disguise".to_owned(),
            artists: "The Wanderers".to_owned(),
            album: "Hard to Find".to_owned(),
            cover_url: None,
            duration: Duration::from_secs(349),
        },
        progress: Duration::from_secs(192),
        playing: true,
        read_at: Instant::now(),
        source: Source::Local,
    }
}

/// `192 s` as `3:12`.
fn clock(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn progress_bar(env: Env, fraction: Memo<f32>, scale: Scale) -> impl View {
    let palette = env.ambient.palette;
    canvas(move |runtime, size, list| {
        let palette = palette.get(runtime);
        let radius = CornerRadii::uniform(size.height / 2.0);
        let track = Rect::from_xywh(0.0, 0.0, size.width, size.height);
        list.fill(
            track,
            radius,
            Paint::solid(palette.ink.with_alpha_factor(0.14)),
        );
        let played = Rect::from_xywh(0.0, 0.0, size.width * fraction.get(runtime), size.height);
        list.fill(played, radius, Paint::solid(palette.ink));
    })
    .w_full()
    .h(scale.px(BAR_HEIGHT))
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let _: Settings = spec.decode()?;
    let playback = cx.signal(None::<Playback>);
    if env.is_live() {
        music::follow(cx, playback);
    } else {
        playback.set(cx, Some(sample()));
    }
    let now = env.ambient.now;
    let position = cx.memo(move |runtime| {
        now.with(runtime, |_| ());
        playback.with(runtime, |playback| {
            playback
                .as_ref()
                .map(|playback| (playback.position(Instant::now()), playback.track.duration))
        })
    });
    let track = cx.memo(move |runtime| {
        playback.with(runtime, |playback| {
            playback.as_ref().map(|playback| playback.track.clone())
        })
    });
    let fraction = cx.memo(move |runtime| {
        position.get(runtime).map_or(0.0, |(at, length)| {
            (at.as_secs_f32() / length.as_secs_f32().max(1.0)).clamp(0.0, 1.0)
        })
    });
    let scale = Scale::new(cx, env.size, measure);
    let top = hstack((
        cover::view(cx, track, env, scale),
        lines::view(track, position, env, scale),
    ))
    .w_full()
    .gap(scale.px(14.0))
    .items_start();
    let card = vstack((
        top,
        spacer(),
        progress_bar(env, fraction, scale),
        vstack(controls::view(playback, position, env, scale)).mt(scale.px(12.0)),
    ))
    .w_full()
    .h_full()
    .p(scale.px(18.0))
    .rounded(scale.px(ROUNDING))
    .overflow_hidden();
    Ok(glass(card, env, |palette| palette.frost).boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_as_minutes_and_seconds() {
        assert_eq!(clock(Duration::from_secs(192)), "3:12");
        assert_eq!(clock(Duration::from_secs(349)), "5:49");
        assert_eq!(clock(Duration::from_secs(5)), "0:05");
    }
}
