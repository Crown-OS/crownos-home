use std::time::Duration;

use serde::Deserialize;

use crownhome_util::uri::encode_component;

use super::{City, Conditions, Forecast, ForecastError, Hour, Unit};

const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING: &str = "https://geocoding-api.open-meteo.com/v1/search";
const TIMEOUT: Duration = Duration::from_secs(15);
const HOURS_SHOWN: usize = 3;

#[derive(Deserialize)]
struct Response {
    current: Current,
    daily: Daily,
    #[serde(default)]
    hourly: Hourly,
}

#[derive(Deserialize)]
struct Current {
    #[serde(default)]
    time: String,
    temperature_2m: f64,
    weather_code: u8,
    is_day: u8,
}

#[derive(Deserialize)]
struct Daily {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

#[derive(Deserialize, Default)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    weather_code: Vec<u8>,
    is_day: Vec<u8>,
}

#[derive(Deserialize)]
struct Places {
    #[serde(default)]
    results: Vec<Place>,
}

#[derive(Deserialize)]
struct Place {
    latitude: f64,
    longitude: f64,
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

fn get(agent: &ureq::Agent, url: &str) -> Result<String, ureq::Error> {
    agent.get(url).call()?.body_mut().read_to_string()
}

pub fn locate(agent: &ureq::Agent, city: &City) -> Result<(f64, f64), ForecastError> {
    if let (Some(latitude), Some(longitude)) = (city.latitude, city.longitude) {
        return Ok((latitude, longitude));
    }
    let url = format!(
        "{GEOCODING}?count=1&format=json&name={}",
        encode_component(&city.name)
    );
    let places: Places = serde_json::from_str(&get(agent, &url)?)?;
    places
        .results
        .first()
        .map(|place| (place.latitude, place.longitude))
        .ok_or_else(|| ForecastError::UnknownPlace(city.name.clone()))
}

pub fn fetch(
    agent: &ureq::Agent,
    (latitude, longitude): (f64, f64),
    unit: Unit,
) -> Result<Forecast, ForecastError> {
    let unit = match unit {
        Unit::Celsius => "celsius",
        Unit::Fahrenheit => "fahrenheit",
    };
    let url = format!(
        "{FORECAST}?latitude={latitude}&longitude={longitude}\
         &current=temperature_2m,weather_code,is_day\
         &hourly=temperature_2m,weather_code,is_day&forecast_hours={}\
         &daily=temperature_2m_max,temperature_2m_min\
         &forecast_days=1&timezone=auto&temperature_unit={unit}",
        HOURS_SHOWN + 1
    );
    parse(&get(agent, &url)?)
}

/// The hour of an Open-Meteo local time such as `2026-05-24T09:00`.
fn hour_of(time: &str) -> Option<i8> {
    time.get(11..13)?.parse().ok()
}

/// The hours after `now`, at most [`HOURS_SHOWN`] of them.
fn coming_hours(hourly: &Hourly, now: &str) -> Vec<Hour> {
    hourly
        .time
        .iter()
        .zip(&hourly.temperature_2m)
        .zip(hourly.weather_code.iter().zip(&hourly.is_day))
        .filter(|((time, _), _)| time.as_str() > now)
        .filter_map(|((time, &temperature), (&code, &is_day))| {
            Some(Hour {
                hour: hour_of(time)?,
                temperature,
                conditions: Conditions {
                    code,
                    daytime: is_day != 0,
                },
            })
        })
        .take(HOURS_SHOWN)
        .collect()
}

fn parse(body: &str) -> Result<Forecast, ForecastError> {
    let response: Response = serde_json::from_str(body)?;
    let (Some(&high), Some(&low)) = (
        response.daily.temperature_2m_max.first(),
        response.daily.temperature_2m_min.first(),
    ) else {
        return Err(ForecastError::NoRange);
    };
    Ok(Forecast {
        temperature: response.current.temperature_2m,
        high,
        low,
        now: Conditions {
            code: response.current.weather_code,
            daytime: response.current.is_day != 0,
        },
        hours: coming_hours(&response.hourly, &response.current.time),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weather::Sky;

    #[test]
    fn an_open_meteo_response_reads_as_a_forecast() -> Result<(), ForecastError> {
        let body = r#"{"latitude":48.14,"current":{"time":"2026-05-24T08:15","temperature_2m":18.3,"weather_code":2,"is_day":1},
            "hourly":{"time":["2026-05-24T08:00","2026-05-24T09:00","2026-05-24T10:00","2026-05-24T11:00"],
                "temperature_2m":[18.0,19.4,20.1,21.0],"weather_code":[2,61,3,0],"is_day":[1,1,1,1]},
            "daily":{"time":["2026-05-24"],"temperature_2m_max":[26.1],"temperature_2m_min":[3.2]}}"#;
        let forecast = parse(body)?;
        assert_eq!(forecast.now.sky(), Sky::PartlyCloudy);
        assert_eq!(
            (forecast.temperature, forecast.high, forecast.low),
            (18.3, 26.1, 3.2)
        );
        assert!(forecast.now.daytime);
        let hours: Vec<_> = forecast.hours.iter().map(|hour| hour.hour).collect();
        assert_eq!(hours, [9, 10, 11]);
        assert_eq!(forecast.hours[0].conditions.sky(), Sky::Rain);
        Ok(())
    }

    #[test]
    fn a_forecast_without_a_daily_range_is_refused() {
        let body = r#"{"current":{"temperature_2m":1,"weather_code":0,"is_day":0},"daily":{"temperature_2m_max":[],"temperature_2m_min":[]}}"#;
        assert!(matches!(parse(body), Err(ForecastError::NoRange)));
    }
}
