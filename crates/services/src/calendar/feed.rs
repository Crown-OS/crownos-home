use std::time::Duration;

use crownos_accounts::AccountError;
use crownui::prelude::{Cx, Signal};

use super::{Agenda, CalendarError, google};

const REFRESH: Duration = Duration::from_secs(5 * 60);
const RETRY: Duration = Duration::from_secs(60);

async fn request() -> Result<Agenda, CalendarError> {
    tokio::task::spawn_blocking(google::coming_week)
        .await
        .map_err(|_| CalendarError::Abandoned)?
}

/// Keeps `shown` on the coming week while the widget is on the board.
pub fn follow(cx: &mut Cx, shown: Signal<Option<Agenda>>) {
    cx.spawn(request(), move |cx, result| {
        let wait = match result {
            Ok(agenda) => {
                shown.set(cx, Some(agenda));
                REFRESH
            }
            Err(CalendarError::Account(AccountError::SignedOut(_))) => {
                shown.set(cx, Some(Agenda::SignedOut));
                RETRY
            }
            Err(e) => {
                log::warn!("calendar: {e}");
                RETRY
            }
        };
        cx.spawn(
            async move { tokio::time::sleep(wait).await },
            move |cx, ()| follow(cx, shown),
        );
    });
}
