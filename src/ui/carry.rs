//! The drag handlers shared by widgets on the board and cards in the gallery.

use crownui::{
    ext::DragEvent,
    prelude::{Cx, Event, Listeners},
};

use crate::ui::{
    desk::Desk,
    gallery,
    gesture::{Gesture, Grip},
};

fn begin(cx: &mut Cx, desk: Desk, grip: Grip) {
    if let Grip::Move(id) = grip {
        desk.raise(cx, id);
    }
    let from = desk.pointer.get(cx);
    desk.gesture.set(cx, Some(Gesture::new(grip, from)));
}

fn travel(cx: &mut Cx, desk: Desk, event: &DragEvent) {
    desk.gesture.update(cx, |gesture| {
        if let Some(gesture) = gesture {
            gesture.travel = event.total;
        }
    });
}

fn finish(cx: &mut Cx, desk: Desk) {
    let Some(gesture) = desk.gesture.get(cx) else {
        return;
    };
    desk.gesture.set(cx, None);
    let grid = desk.board_grid(cx);
    let over_gallery = gallery::covers(cx, desk, gesture.pointer());
    let outcome = desk
        .tiles
        .with(cx, |tiles| gesture.release(tiles, &grid, over_gallery));
    desk.apply(cx, outcome);
}

/// Makes `element` carry what `grip` names when it is dragged.
pub fn carries<E: Listeners>(element: E, desk: Desk, grip: Grip) -> E {
    element
        .on_drag_start(move |cx, _: &mut Event<DragEvent>| begin(cx, desk, grip.clone()))
        .on_drag(move |cx, event| travel(cx, desk, event))
        .on_drag_end(move |cx, _| finish(cx, desk))
}
