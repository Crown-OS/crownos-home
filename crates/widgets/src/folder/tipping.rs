//! Springs the front's tip toward where the folder's stance wants it, frame by
//! frame on the UI thread, since bending the pane's outline is not a transform
//! the render thread can animate.

use crownui::prelude::{Cx, Signal, Spring, Styled, View, keyed, vstack};
use motion::Animation;

use super::{Stance, State, front::Tip};

/// How far the front tips while the papers stand lifted out of it.
pub const LIFTED: f32 = 0.5;

/// The front's tip as it is now, and the view that keeps it moving.
pub fn follow(cx: &mut Cx, state: State) -> (Signal<Tip>, impl View + use<>) {
    let shown = cx.signal(Tip(0.0));
    let segment = cx.signal(None::<Animation<f32>>);
    let driver = keyed(
        move |runtime| state.stance(runtime),
        move |cx, stance| {
            step(cx, wanted(stance), shown, segment);
            vstack(())
        },
    )
    .absolute();
    (shown, driver)
}

fn wanted(stance: Stance) -> f32 {
    match stance {
        Stance::Resting => 0.0,
        Stance::Lifted => LIFTED,
        Stance::Spread => 1.0,
    }
}

fn step(cx: &mut Cx, goal: f32, shown: Signal<Tip>, segment: Signal<Option<Animation<f32>>>) {
    cx.request_animation_frame(move |cx, now| {
        let curve = Spring::snappy().into();
        let running = match segment.get(cx) {
            Some(running) if running.target() == goal => running,
            Some(running) => running.retarget(now, goal, curve),
            None if shown.get(cx).0 == goal => return,
            None => Animation::new(shown.get(cx).0, goal, 0.0, now, curve),
        };
        let sample = running.sample(now);
        shown.set(cx, Tip(sample.value));
        if sample.finished {
            segment.set(cx, None);
        } else {
            segment.set(cx, Some(running));
            step(cx, goal, shown, segment);
        }
    });
}
