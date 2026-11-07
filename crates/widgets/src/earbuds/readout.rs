use crownui::prelude::{
    Color, IntoProp, Runtime, Signal, Styled, View, WHITE, bind, hstack, spacer, text, vstack,
};

use crate::scale::Scale;
use crownhome_services::earbuds::Earbuds;

const INSET: (f32, f32) = (17.0, 19.0);
const HEADING_SIZE: f32 = 9.0;
const CAPTION_SIZE: f32 = 10.5;
const CHARGE_SIZE: f32 = 24.0;
const NAME_SIZE: f32 = 20.0;
const CAPTION_ALPHA: f32 = 0.86;
const UNKNOWN: &str = "--";

fn caption(line: impl IntoProp<String>, scale: Scale) -> impl View {
    text(line)
        .text_size(scale.px(CAPTION_SIZE))
        .text_color(Color::new(1.0, 1.0, 1.0, CAPTION_ALPHA))
}

/// `82%`, or `--%` while the earbuds do not say.
fn charge(battery: Option<u8>) -> String {
    battery.map_or_else(|| format!("{UNKNOWN}%"), |percent| format!("{percent}%"))
}

fn status(earbuds: Option<&Earbuds>) -> &'static str {
    match earbuds {
        Some(earbuds) if earbuds.connected => "Connected to:",
        Some(_) => "Not connected:",
        None => "No earbuds paired",
    }
}

pub fn view(earbuds: Signal<Option<Earbuds>>, scale: Scale) -> impl View {
    let read = move |pick: fn(Option<&Earbuds>) -> String| {
        bind(move |runtime: &mut Runtime| earbuds.with(runtime, |earbuds| pick(earbuds.as_ref())))
    };
    let heading = text("BATTERY:")
        .text_size(scale.px(HEADING_SIZE))
        .font_weight(500)
        .tracking(scale.px(0.2))
        .text_color(WHITE);
    let level = vstack((
        caption("Earbuds:", scale),
        text(read(|earbuds| {
            charge(earbuds.and_then(|earbuds| earbuds.battery))
        }))
        .text_size(scale.px(CHARGE_SIZE))
        .font_weight(500)
        .tracking(scale.px(-0.6))
        .text_color(WHITE),
    ))
    .items_start();
    let name = text(read(|earbuds| {
        earbuds
            .map(|earbuds| earbuds.name.clone())
            .unwrap_or_default()
    }))
    .text_size(scale.px(NAME_SIZE))
    .font_weight(560)
    .tracking(scale.px(-0.5))
    .line_clamp(2)
    .text_color(WHITE);
    vstack((
        hstack((heading, spacer(), level)).w_full().items_start(),
        spacer(),
        vstack((
            caption(read(|earbuds| status(earbuds).to_owned()), scale),
            name,
        ))
        .gap(scale.px(2.0)),
    ))
    .absolute()
    .inset(0.0)
    .px(scale.px(INSET.0))
    .py(scale.px(INSET.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreported_charge_reads_as_dashes() {
        assert_eq!(charge(Some(80)), "80%");
        assert_eq!(charge(None), "--%");
    }

    #[test]
    fn the_caption_says_whether_the_earbuds_are_connected() {
        let earbuds = |connected| Earbuds {
            name: "Buds".to_owned(),
            connected,
            battery: None,
        };
        assert_eq!(status(Some(&earbuds(true))), "Connected to:");
        assert_eq!(status(Some(&earbuds(false))), "Not connected:");
        assert_eq!(status(None), "No earbuds paired");
    }
}
