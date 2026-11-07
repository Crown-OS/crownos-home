//! Keeps a weather widget's forecast current while the widget is on the board.
//!
//! The request blocks, so it runs on tokio's blocking pool; the loop is a
//! chain of tasks owned by the widget, and ends when the widget is removed.

use std::time::Duration;

use crownui::prelude::{Cx, Signal};

use super::{City, Forecast, ForecastError, Unit, agent, fetch, locate};

const REFRESH: Duration = Duration::from_secs(30 * 60);
const RETRY: Duration = Duration::from_secs(5 * 60);

pub fn follow(cx: &mut Cx, city: City, unit: Unit, shown: Signal<Option<Forecast>>) {
    cx.spawn(request(city.clone(), unit), move |cx, result| {
        let wait = match result {
            Ok(fresh) => {
                shown.set(cx, Some(fresh));
                REFRESH
            }
            Err(e) => {
                log::warn!("weather for {}: {e}", city.name);
                RETRY
            }
        };
        cx.spawn(
            async move { tokio::time::sleep(wait).await },
            move |cx, ()| {
                follow(cx, city, unit, shown);
            },
        );
    });
}

async fn request(city: City, unit: Unit) -> Result<Forecast, ForecastError> {
    tokio::task::spawn_blocking(move || {
        let agent = agent();
        let place = locate(&agent, &city)?;
        fetch(&agent, place, unit)
    })
    .await
    .map_err(|_| ForecastError::Abandoned)?
}
