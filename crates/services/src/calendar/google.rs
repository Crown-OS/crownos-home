use std::sync::LazyLock;

use crownhome_util::uri::encode_component;
use crownos_accounts::{Provider, access_token};
use jiff::{SignedDuration, Timestamp, Zoned, civil::Date};
use serde::Deserialize;

use super::{Agenda, CalendarError, Event, When};

const API: &str = "https://www.googleapis.com/calendar/v3";
const DAYS_AHEAD: i64 = 7;
const EVENTS_PER_CALENDAR: u32 = 25;

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(crownos_accounts::agent);

#[derive(Deserialize)]
struct CalendarList {
    #[serde(default)]
    items: Vec<Calendar>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Calendar {
    id: String,
    background_color: Option<String>,
    #[serde(default)]
    selected: bool,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct Events {
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    summary: Option<String>,
    start: Moment,
    end: Moment,
    location: Option<String>,
    hangout_link: Option<String>,
    #[serde(default)]
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Moment {
    date_time: Option<String>,
    date: Option<String>,
}

fn get<T: serde::de::DeserializeOwned>(url: &str, bearer: &str) -> Result<T, CalendarError> {
    let body = AGENT
        .get(url)
        .header("Authorization", bearer)
        .call()?
        .body_mut()
        .read_to_string()?;
    Ok(serde_json::from_str(&body)?)
}

fn when(start: &Moment, end: &Moment) -> Result<Option<When>, CalendarError> {
    Ok(match (&start.date_time, &end.date_time, &start.date) {
        (Some(start), Some(end), _) => Some(When::Timed {
            start: start.parse::<Timestamp>()?,
            end: end.parse::<Timestamp>()?,
        }),
        (_, _, Some(day)) => Some(When::AllDay(day.parse::<Date>()?)),
        _ => None,
    })
}

fn event(item: Item, color: Option<&str>) -> Result<Option<Event>, CalendarError> {
    if item.status == "cancelled" {
        return Ok(None);
    }
    let Some(when) = when(&item.start, &item.end)? else {
        return Ok(None);
    };
    Ok(Some(Event {
        title: item.summary.unwrap_or_else(|| "(No title)".to_owned()),
        when,
        location: item.location.filter(|location| !location.is_empty()),
        color: color.map(str::to_owned),
        meeting_url: item.hangout_link,
    }))
}

fn starts(event: &Event) -> Timestamp {
    match &event.when {
        When::Timed { start, .. } => *start,
        When::AllDay(day) => day
            .to_zoned(jiff::tz::TimeZone::system())
            .map_or(Timestamp::UNIX_EPOCH, |zoned| zoned.timestamp()),
    }
}

fn calendar_events(
    calendar: &Calendar,
    bearer: &str,
    (from, until): (&Timestamp, &Timestamp),
) -> Result<Vec<Event>, CalendarError> {
    let url = format!(
        "{API}/calendars/{}/events?singleEvents=true&orderBy=startTime&maxResults={EVENTS_PER_CALENDAR}&timeMin={}&timeMax={}",
        encode_component(&calendar.id),
        encode_component(&from.to_string()),
        encode_component(&until.to_string()),
    );
    let events: Events = get(&url, bearer)?;
    events
        .items
        .into_iter()
        .filter_map(|item| event(item, calendar.background_color.as_deref()).transpose())
        .collect()
}

/// Every shown calendar's events from the start of today to a week on.
pub fn coming_week() -> Result<Agenda, CalendarError> {
    let bearer = format!("Bearer {}", access_token(&AGENT, Provider::Google)?);
    let calendars: CalendarList = get(&format!("{API}/users/me/calendarList"), &bearer)?;
    let today = Zoned::now().start_of_day()?;
    let from = today.timestamp();
    let until = from + SignedDuration::from_hours(24 * DAYS_AHEAD);
    let mut events = calendars
        .items
        .iter()
        .filter(|calendar| calendar.selected || calendar.primary)
        .map(|calendar| calendar_events(calendar, &bearer, (&from, &until)))
        .collect::<Result<Vec<_>, _>>()?
        .concat();
    events.sort_by_key(starts);
    Ok(Agenda::Events(events))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_meeting_reads_with_its_link_and_colour() -> Result<(), CalendarError> {
        let body = r##"{"items":[
            {"summary":"Design review","status":"confirmed","hangoutLink":"https://meet.google.com/abc",
             "start":{"dateTime":"2026-11-05T08:15:00+05:30"},"end":{"dateTime":"2026-11-05T09:00:00+05:30"}},
            {"summary":"Gone","status":"cancelled","start":{"date":"2026-11-05"},"end":{"date":"2026-11-06"}},
            {"summary":"Take the pill","status":"confirmed","start":{"date":"2026-11-05"},"end":{"date":"2026-11-06"}}
        ]}"##;
        let events: Events = serde_json::from_str(body)?;
        let read: Vec<_> = events
            .items
            .into_iter()
            .filter_map(|item| event(item, Some("#7986cb")).transpose())
            .collect::<Result<_, _>>()?;
        assert_eq!(read.len(), 2);
        assert_eq!(
            read[0].meeting_url.as_deref(),
            Some("https://meet.google.com/abc")
        );
        assert_eq!(read[0].color.as_deref(), Some("#7986cb"));
        let When::Timed { start, end } = read[0].when else {
            panic!("a timed meeting");
        };
        assert_eq!(end.duration_since(start), SignedDuration::from_mins(45));
        assert!(read[1].is_all_day());
        Ok(())
    }
}
