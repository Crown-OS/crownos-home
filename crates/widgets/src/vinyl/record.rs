use std::{cell::Cell, rc::Rc, time::Instant};

use crownui::prelude::{Curve, Cx, Memo, Signal, Styled, View, keyed, vstack, zstack};

use crate::{art, scale::Scale};
use crownhome_services::music::Playback;

const RECORD: &[u8] = include_bytes!("../../assets/vinyl/record.svg");
const SHEEN: &[u8] = include_bytes!("../../assets/vinyl/sheen.svg");
const SHADOW: &[u8] = include_bytes!("../../assets/vinyl/shadow.svg");
/// The shadow's picture reaches this far past the record on every side.
const SHADOW_MARGIN: f32 = 0.1 * 2.0 * RADIUS;
const CENTRE: (f32, f32) = (500.0, 480.0);
const RADIUS: f32 = 425.0;
/// How far below its resting place a new record starts, inside the sleeve.
const SLEEVE_DEPTH: f32 = 520.0;
const RISE_MILLIS: u64 = 650;
/// One turn at 33⅓ rpm.
const TURN_MILLIS: u64 = 1800;
const FULL_TURN: f32 = 360.0;

/// Where the record stands, in degrees, and since when it has been turning.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Turntable {
    angle: f32,
    spinning_since: Option<Instant>,
}

impl Turntable {
    fn angle_at(self, now: Instant) -> f32 {
        let turns = self.spinning_since.map_or(0.0, |since| {
            now.duration_since(since).as_millis() as f32 / TURN_MILLIS as f32
        });
        (self.angle + turns.fract() * FULL_TURN) % FULL_TURN
    }

    /// Starts or stops the record at `now`, returning the angle it is at.
    fn set_spinning(&mut self, spinning: bool, now: Instant) -> f32 {
        let angle = self.angle_at(now);
        *self = Self {
            angle,
            spinning_since: spinning.then_some(now),
        };
        angle
    }
}

/// The disc turns on the render thread; a stopped one keeps the angle it stopped at.
fn disc(playing: Memo<bool>, diameter: Memo<f32>) -> impl View {
    let turntable = Rc::new(Cell::new(Turntable::default()));
    keyed(
        move |runtime| playing.get(runtime),
        move |cx, playing| {
            let mut table = turntable.get();
            let angle = table.set_spinning(playing, Instant::now());
            turntable.set(table);
            let record = vstack(art::picture(cx, |_| RECORD, diameter));
            if playing {
                record
                    .rotate(angle + FULL_TURN)
                    .enter(move |style| style.rotate(angle))
                    .transition(Curve::linear(TURN_MILLIS).repeating())
            } else {
                record.rotate(angle)
            }
        },
    )
}

/// The record and the light on it, rising out of the sleeve whenever the track changes.
pub fn view(cx: &mut Cx, playback: Signal<Option<Playback>>, scale: Scale) -> impl View + use<> {
    let diameter = scale.length(cx, 2.0 * RADIUS);
    let shadow_edge = scale.length(cx, 2.0 * (RADIUS + SHADOW_MARGIN));
    let factor = scale.factor();
    let track = cx.memo(move |runtime| {
        playback.with(runtime, |playback| {
            playback
                .as_ref()
                .map(|playback| (playback.track.title.clone(), playback.track.artists.clone()))
        })
    });
    let playing = cx.memo(move |runtime| {
        playback.with(runtime, |playback| {
            playback.as_ref().is_some_and(|playback| playback.playing)
        })
    });
    keyed(
        move |runtime| track.get(runtime),
        move |cx, _| {
            let depth = factor.get(cx) * SLEEVE_DEPTH;
            let sheen = vstack(art::picture(cx, |_| SHEEN, diameter))
                .absolute()
                .inset(0.0);
            let shadow = vstack(art::picture(cx, |_| SHADOW, shadow_edge))
                .absolute()
                .left(scale.px(-SHADOW_MARGIN))
                .top(scale.px(-SHADOW_MARGIN));
            zstack((shadow, disc(playing, diameter), sheen))
                .relative()
                .translate_y(0.0)
                .enter(move |style| style.translate_y(depth))
                .transition(Curve::ease_out(RISE_MILLIS))
        },
    )
    .absolute()
    .left(scale.px(CENTRE.0 - RADIUS))
    .top(scale.px(CENTRE.1 - RADIUS))
    .w(diameter)
    .h(diameter)
    .pointer_events_none()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_stopped_record_keeps_its_angle() {
        let start = Instant::now();
        let mut table = Turntable::default();
        assert_eq!(table.set_spinning(true, start), 0.0);
        let quarter = start + Duration::from_millis(TURN_MILLIS / 4);
        assert!((table.set_spinning(false, quarter) - 90.0).abs() < 0.5);
        let later = quarter + Duration::from_secs(10);
        assert!((table.angle_at(later) - 90.0).abs() < 0.5);
    }

    #[test]
    fn whole_turns_come_back_to_the_start() {
        let start = Instant::now();
        let table = Turntable {
            angle: 30.0,
            spinning_since: Some(start),
        };
        let three_turns = start + Duration::from_millis(3 * TURN_MILLIS);
        assert!((table.angle_at(three_turns) - 30.0).abs() < 0.5);
    }
}
