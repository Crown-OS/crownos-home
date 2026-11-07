//! The Bluetooth earbuds paired here: what they are called, whether they are
//! connected, and how much charge they report.

mod bluez;

use std::time::Duration;

use crownui::prelude::{Cx, Signal};

/// How often BlueZ is asked again.
const POLL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Earbuds {
    pub name: String,
    pub connected: bool,
    /// The charge the earbuds report, in percent; BlueZ gives one figure for the pair.
    pub battery: Option<u8>,
}

/// The earbuds to show: connected ones before the rest, then the ones that
/// report their charge, then by name so the choice does not wander.
fn choose(paired: impl IntoIterator<Item = Earbuds>) -> Option<Earbuds> {
    paired.into_iter().min_by(|first, second| {
        (second.connected, second.battery.is_some())
            .cmp(&(first.connected, first.battery.is_some()))
            .then_with(|| first.name.cmp(&second.name))
    })
}

pub async fn current() -> Option<Earbuds> {
    bluez::paired()
        .await
        .map_err(|e| log::debug!("earbuds: {e}"))
        .ok()
        .and_then(choose)
}

/// Keeps `shown` on the earbuds while the widget is on the board.
pub fn follow(cx: &mut Cx, shown: Signal<Option<Earbuds>>) {
    cx.spawn(current(), move |cx, earbuds| {
        shown.set(cx, earbuds);
        cx.spawn(
            async move { tokio::time::sleep(POLL).await },
            move |cx, ()| follow(cx, shown),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn earbuds(name: &str, connected: bool, battery: Option<u8>) -> Earbuds {
        Earbuds {
            name: name.to_owned(),
            connected,
            battery,
        }
    }

    #[test]
    fn connected_earbuds_come_before_the_rest() {
        let chosen = choose([earbuds("Old", false, Some(40)), earbuds("Worn", true, None)]);
        assert_eq!(chosen.map(|earbuds| earbuds.name), Some("Worn".to_owned()));
    }

    #[test]
    fn among_equals_the_first_name_wins() {
        let chosen = choose([earbuds("Zed", false, None), earbuds("Ace", false, None)]);
        assert_eq!(chosen.map(|earbuds| earbuds.name), Some("Ace".to_owned()));
    }

    #[test]
    fn nothing_paired_is_nothing_to_show() {
        assert_eq!(choose([]), None);
    }
}
