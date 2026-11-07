use std::{rc::Rc, time::Duration};

use crownui::prelude::{Cx, Memo, Runtime, Signal, Styled, View, bind, keyed, text, vstack};

use crate::{
    Env,
    scale::{Scale, ink},
};
use crownhome_services::music::{Lyrics, Track, fetch_lyrics};

const SAMPLE_LYRICS: &str = "[00:00.00] I can see it's hard to find\n[03:00.00] This blessing in disguise\n[03:30.00] This blessing in disguise";
const NOTHING_PLAYING: [&str; 3] = ["", "Not Playing", "Spotify"];

/// The three lines shown at `at`: the one just sung, the one being sung, and
/// the next. Without lyrics they are the album, the title and the artists.
fn shown(track: Option<&Track>, lyrics: Option<&Lyrics>, at: Duration) -> [String; 3] {
    let Some(track) = track else {
        return NOTHING_PLAYING.map(str::to_owned);
    };
    match lyrics.map(|lyrics| lyrics.around(at)) {
        Some([previous, Some(current), next]) => {
            [previous, Some(current), next].map(|line| line.unwrap_or_default().to_owned())
        }
        Some([_, None, next]) => [
            String::new(),
            track.title.clone(),
            next.unwrap_or_default().to_owned(),
        ],
        None => [
            track.album.clone(),
            track.title.clone(),
            track.artists.clone(),
        ],
    }
}

fn column(
    cx: &mut Cx,
    track: Option<Track>,
    position: Memo<Option<(Duration, Duration)>>,
    env: Env,
    scale: Scale,
) -> impl View + use<> {
    let lyrics: Signal<Option<Lyrics>> = cx.signal(None);
    match (&track, env.is_live()) {
        (Some(track), true) => {
            cx.spawn(fetch_lyrics(track.clone()), move |cx, found| {
                lyrics.set(cx, found);
            });
        }
        (Some(_), false) => lyrics.set(cx, Some(Lyrics::parse(SAMPLE_LYRICS))),
        (None, _) => {}
    }
    let track = Rc::new(track);
    let line = |slot: usize| {
        let track = Rc::clone(&track);
        bind(move |runtime: &mut Runtime| {
            let at = position.get(runtime).map_or(Duration::ZERO, |(at, _)| at);
            lyrics.with(runtime, |lyrics| {
                shown(track.as_ref().as_ref(), lyrics.as_ref(), at)
                    .into_iter()
                    .nth(slot)
                    .unwrap_or_default()
            })
        })
    };
    let palette = env.ambient.palette;
    let faint = |slot: usize| {
        text(line(slot))
            .text_size(scale.px(14.0))
            .line_clamp(2)
            .text_color(ink(palette, |palette| {
                palette.ink_secondary.with_alpha_factor(0.7)
            }))
    };
    let sung = text(line(1))
        .text_size(scale.px(16.0))
        .font_weight(560)
        .line_clamp(2)
        .text_color(ink(palette, |palette| palette.ink));
    vstack((faint(0), sung, faint(2)))
        .gap(scale.px(3.0))
        .w_full()
}

pub fn view(
    track: Memo<Option<Track>>,
    position: Memo<Option<(Duration, Duration)>>,
    env: Env,
    scale: Scale,
) -> impl View {
    keyed(
        move |runtime| track.get(runtime),
        move |cx, track| column(cx, track, position, env, scale),
    )
    .shrink(1.0)
    .min_w(0.0)
    .grow(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> Track {
        Track {
            title: "Title".to_owned(),
            artists: "Artist".to_owned(),
            album: "Album".to_owned(),
            cover_url: None,
            duration: Duration::from_secs(100),
        }
    }

    #[test]
    fn without_lyrics_the_lines_name_the_track() {
        let [album, title, artists] = shown(Some(&track()), None, Duration::ZERO);
        assert_eq!(
            (album.as_str(), title.as_str(), artists.as_str()),
            ("Album", "Title", "Artist")
        );
    }

    #[test]
    fn before_the_first_line_the_title_waits_above_it() {
        let lyrics = Lyrics::parse("[00:10.00] first");
        let [_, title, next] = shown(Some(&track()), Some(&lyrics), Duration::ZERO);
        assert_eq!((title.as_str(), next.as_str()), ("Title", "first"));
    }

    #[test]
    fn nothing_playing_says_so() {
        assert_eq!(shown(None, None, Duration::ZERO)[1], "Not Playing");
    }
}
