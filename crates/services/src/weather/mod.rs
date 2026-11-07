//! Weather for one place, from Open-Meteo, which needs no key.

mod feed;
mod open_meteo;

use serde::{Deserialize, Serialize};

pub use feed::follow;
pub use open_meteo::{agent, fetch, locate};

#[derive(Debug, Clone, PartialEq)]
pub struct City {
    pub name: String,
    /// Looked up by name when either coordinate is missing.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sky {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Rain,
    Snow,
    Thunder,
}

/// The sky at one moment, as a WMO weather interpretation code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conditions {
    pub code: u8,
    pub daytime: bool,
}

impl Conditions {
    pub fn sky(self) -> Sky {
        match self.code {
            0 => Sky::Clear,
            1 | 2 => Sky::PartlyCloudy,
            45 | 48 => Sky::Fog,
            51..=67 | 80..=82 => Sky::Rain,
            71..=77 | 85 | 86 => Sky::Snow,
            95..=99 => Sky::Thunder,
            _ => Sky::Cloudy,
        }
    }

    pub fn description(self) -> &'static str {
        match self.code {
            0 if self.daytime => "Sunny",
            0 => "Clear",
            1 => "Mostly Clear",
            2 => "Partly Cloudy",
            3 => "Mostly Cloudy",
            45 | 48 => "Foggy",
            51..=57 => "Drizzle",
            61..=67 => "Rain",
            71..=77 => "Snow",
            80..=82 => "Showers",
            85 | 86 => "Snow Showers",
            95..=99 => "Thunderstorms",
            _ => "Cloudy",
        }
    }
}

/// The forecast for one coming hour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hour {
    /// 0 to 23, in the place's own time.
    pub hour: i8,
    pub temperature: f64,
    pub conditions: Conditions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Forecast {
    pub temperature: f64,
    pub high: f64,
    pub low: f64,
    pub now: Conditions,
    pub hours: Vec<Hour>,
}

#[derive(Debug, thiserror::Error)]
pub enum ForecastError {
    #[error("requesting {0}")]
    Request(#[from] ureq::Error),
    #[error("reading the response")]
    Decode(#[from] serde_json::Error),
    #[error("no place called {0}")]
    UnknownPlace(String),
    #[error("the forecast has no daily range")]
    NoRange,
    #[error("the request was abandoned")]
    Abandoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wmo_codes_group_into_skies() {
        let sky = |code| {
            Conditions {
                code,
                daytime: true,
            }
            .sky()
        };
        assert_eq!(sky(0), Sky::Clear);
        assert_eq!(sky(3), Sky::Cloudy);
        assert_eq!(sky(61), Sky::Rain);
        assert_eq!(sky(81), Sky::Rain);
        assert_eq!(sky(73), Sky::Snow);
        assert_eq!(sky(95), Sky::Thunder);
        assert_eq!(sky(48), Sky::Fog);
    }

    #[test]
    fn a_clear_sky_is_sunny_only_by_day() {
        let clear = |daytime| Conditions { code: 0, daytime }.description();
        assert_eq!(clear(true), "Sunny");
        assert_eq!(clear(false), "Clear");
    }
}
