//! The time every widget reads, moved on at each whole second.

use std::time::Duration;

use crownui::prelude::{Cx, Signal};
use jiff::Zoned;

pub fn start(cx: &mut Cx) -> Signal<Zoned> {
    let now = cx.signal(Zoned::now());
    tick(cx, now);
    now
}

fn tick(cx: &mut Cx, now: Signal<Zoned>) {
    let past = now
        .with(cx, |now| now.subsec_nanosecond())
        .clamp(0, 999_999_999);
    let wait = Duration::from_nanos(1_000_000_000 - past.unsigned_abs() as u64);
    cx.spawn(
        async move { tokio::time::sleep(wait).await },
        move |cx, ()| {
            now.set(cx, Zoned::now());
            tick(cx, now);
        },
    );
}
