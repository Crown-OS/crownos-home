//! The app launcher, crownpad, which the search field opens.

use std::time::Duration;

const TIMEOUT: Duration = Duration::from_millis(500);

crownos_ipc::protocol! {
    pub mod pad: "crownpad" {
        fn show() -> bool;
        fn hide() -> bool;
        fn toggle() -> bool;
        fn is_visible() -> bool;
    }
}

pub async fn show() {
    let asked = tokio::task::spawn_blocking(|| {
        pad::Client::connect().and_then(|mut client| {
            client.call_blocking::<pad::show>(&pad::show {}, Vec::new(), Some(TIMEOUT))
        })
    })
    .await;
    match asked {
        Ok(Err(e)) => log::warn!("opening crownpad: {e}"),
        Err(e) => log::warn!("opening crownpad: {e}"),
        Ok(Ok(_)) => {}
    }
}
