use std::time::Duration;

use crownui::prelude::{Cx, Signal};

use super::{Playback, Source, now_playing};

const LOCAL_POLL: Duration = Duration::from_secs(1);
const WEB_POLL: Duration = Duration::from_secs(3);
const IDLE_POLL: Duration = Duration::from_secs(10);

/// Keeps `shown` on what is playing while the widget is on the board.
pub fn follow(cx: &mut Cx, shown: Signal<Option<Playback>>) {
    cx.spawn(now_playing(), move |cx, playback| {
        let wait = match playback.as_ref().map(|playback| playback.source) {
            Some(Source::Local) => LOCAL_POLL,
            Some(Source::Web) => WEB_POLL,
            None => IDLE_POLL,
        };
        shown.set(cx, playback);
        cx.spawn(
            async move { tokio::time::sleep(wait).await },
            move |cx, ()| follow(cx, shown),
        );
    });
}
