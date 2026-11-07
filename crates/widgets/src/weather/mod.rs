//! The weather for one city: a frosted pill when the widget is one row tall,
//! a sky-tinted card with the coming hours when it has room.

mod card;
mod pill;

use crownui::prelude::{Cx, Memo, Runtime, Signal, Size, Styled, View, keyed};
use serde::{Deserialize, Serialize};

use crate::{Built, Env, Shape};
use crownhome_board::WidgetSpec;
use crownhome_services::weather::{self, City, Conditions, Forecast, Hour, Sky, Unit};

pub const KIND: &str = "weather";

const MISSING: &str = "--";
/// Frames at least this tall for their width get the card.
const CARD_ASPECT: f32 = 0.7;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub unit: Unit,
}

impl Settings {
    fn city(&self) -> City {
        City {
            name: self.name.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

fn sample() -> Forecast {
    let hour = |hour, temperature, code| Hour {
        hour,
        temperature,
        conditions: Conditions {
            code,
            daytime: true,
        },
    };
    Forecast {
        temperature: 17.0,
        high: 21.0,
        low: 12.0,
        now: Conditions {
            code: 3,
            daytime: true,
        },
        hours: vec![hour(7, 16.0, 61), hour(8, 15.0, 61), hour(9, 16.0, 2)],
    }
}

fn degrees(value: f64) -> String {
    format!("{}", value.round() as i64)
}

/// `read` of the forecast once there is one, `missing` until then.
fn reading(
    forecast: Signal<Option<Forecast>>,
    read: impl Fn(&Forecast) -> String + 'static,
    missing: &'static str,
) -> impl FnMut(&mut Runtime) -> String {
    move |runtime| {
        forecast.with(runtime, |forecast| {
            forecast.as_ref().map_or_else(|| missing.to_owned(), &read)
        })
    }
}

fn glyph(conditions: Option<Conditions>) -> &'static [u8] {
    let conditions = conditions.unwrap_or(Conditions {
        code: 3,
        daytime: true,
    });
    match (conditions.sky(), conditions.daytime) {
        (Sky::Clear, true) => include_bytes!("../../assets/weather/clear-day.svg"),
        (Sky::Clear, false) => include_bytes!("../../assets/weather/clear-night.svg"),
        (Sky::PartlyCloudy, true) => {
            include_bytes!("../../assets/weather/partly-cloudy-day.svg")
        }
        (Sky::PartlyCloudy, false) => {
            include_bytes!("../../assets/weather/partly-cloudy-night.svg")
        }
        (Sky::Cloudy, _) => include_bytes!("../../assets/weather/cloudy.svg"),
        (Sky::Fog, _) => include_bytes!("../../assets/weather/fog.svg"),
        (Sky::Rain, _) => include_bytes!("../../assets/weather/rain.svg"),
        (Sky::Snow, _) => include_bytes!("../../assets/weather/snow.svg"),
        (Sky::Thunder, _) => include_bytes!("../../assets/weather/thunder.svg"),
    }
}

fn has_room_for_card(size: Size) -> bool {
    size.height >= size.width * CARD_ASPECT
}

pub fn shape(size: Size) -> Shape {
    if has_room_for_card(size) {
        card::shape(size)
    } else {
        pill::shape(size)
    }
}

pub fn view(cx: &mut Cx, spec: &WidgetSpec, env: Env) -> Built {
    let settings: Settings = spec.decode()?;
    let forecast = cx.signal(None);
    if env.is_live() {
        weather::follow(cx, settings.city(), settings.unit, forecast);
    } else {
        forecast.set(cx, Some(sample()));
    }
    let roomy: Memo<bool> = cx.memo(move |runtime| has_room_for_card(env.size.get(runtime)));
    let name = settings.name;
    Ok(keyed(
        move |runtime| roomy.get(runtime),
        move |cx, roomy| {
            if roomy {
                card::view(cx, &name, forecast, env).boxed()
            } else {
                pill::view(cx, &name, forecast, env).boxed()
            }
        },
    )
    .w_full()
    .h_full()
    .boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperatures_read_as_whole_degrees() {
        assert_eq!(degrees(18.4), "18");
        assert_eq!(degrees(-0.6), "-1");
    }

    #[test]
    fn a_square_frame_gets_the_card_and_a_wide_one_the_pill() {
        assert!(has_room_for_card(Size::new(237.5, 241.0)));
        assert!(!has_room_for_card(Size::new(237.5, 100.0)));
    }

    #[test]
    fn every_sky_has_a_glyph_that_rasterizes() {
        for code in [0, 2, 3, 45, 61, 73, 95] {
            for daytime in [true, false] {
                let svg = glyph(Some(Conditions { code, daytime }));
                let picture = crownhome_util::raster::rasterize_svg(svg, 48);
                assert!(picture.is_some(), "{code} {daytime}");
            }
        }
    }
}
