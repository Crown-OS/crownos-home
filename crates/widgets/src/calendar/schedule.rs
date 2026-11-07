//! What the calendar widget shows at one moment, worked out from the agenda.

use jiff::{SignedDuration, Timestamp, ToSpan, Zoned, civil::Date, tz::TimeZone};

use crownhome_services::calendar::{Agenda, Event, When};

const UPCOMING_SHOWN: usize = 2;

/// One coming meeting, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upcoming {
    pub title: String,
    pub detail: String,
    pub color: Option<String>,
    pub meeting_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coming {
    SignedOut,
    Loading,
    Events(Vec<Upcoming>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub today: Date,
    pub events_today: usize,
    pub all_day: Option<String>,
    pub coming: Coming,
}

fn local_date(at: Timestamp) -> Date {
    at.to_zoned(TimeZone::system()).date()
}

fn falls_on(event: &Event, day: Date) -> bool {
    match &event.when {
        When::AllDay(date) => *date == day,
        When::Timed { start, .. } => local_date(*start) == day,
    }
}

/// `8:15 AM`.
pub fn clock(at: Timestamp) -> String {
    let local = at.to_zoned(TimeZone::system());
    let hour = match local.hour() % 12 {
        0 => 12,
        hour => hour,
    };
    let half = if local.hour() < 12 { "AM" } else { "PM" };
    format!("{hour}:{:02} {half}", local.minute())
}

/// `45 min`, `1 hr`, `1 hr 30 min`.
pub fn length(duration: SignedDuration) -> String {
    let minutes = duration.as_mins().max(0);
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} hr"),
        (hours, minutes) => format!("{hours} hr {minutes} min"),
    }
}

fn upcoming(event: &Event, today: Date) -> Option<Upcoming> {
    let When::Timed { start, end } = event.when else {
        return None;
    };
    let day = local_date(start);
    let mut detail = if day == today {
        clock(start)
    } else {
        format!("{} {}", day.strftime("%a"), clock(start))
    };
    detail.push_str(&format!(" · {}", length(end.duration_since(start))));
    if let Some(location) = &event.location {
        detail.push_str(&format!(" · {location}"));
    }
    Some(Upcoming {
        title: event.title.clone(),
        detail,
        color: event.color.clone(),
        meeting_url: event.meeting_url.clone(),
    })
}

impl Summary {
    pub fn of(agenda: Option<&Agenda>, now: &Zoned) -> Self {
        let today = now.date();
        let events = match agenda {
            Some(Agenda::Events(events)) => events.as_slice(),
            _ => &[],
        };
        let still_ahead = |event: &&Event| match event.when {
            When::Timed { end, .. } => end > now.timestamp(),
            When::AllDay(_) => false,
        };
        let coming = match agenda {
            None => Coming::Loading,
            Some(Agenda::SignedOut) => Coming::SignedOut,
            Some(Agenda::Events(_)) => Coming::Events(
                events
                    .iter()
                    .filter(still_ahead)
                    .filter_map(|event| upcoming(event, today))
                    .take(UPCOMING_SHOWN)
                    .collect(),
            ),
        };
        Self {
            today,
            events_today: events.iter().filter(|event| falls_on(event, today)).count(),
            all_day: events
                .iter()
                .find(|event| event.is_all_day() && falls_on(event, today))
                .map(|event| event.title.clone()),
            coming,
        }
    }

    /// Sunday to Saturday of the week holding today.
    pub fn week(&self) -> [Date; 7] {
        let sunday = self
            .today
            .saturating_sub(i64::from(self.today.weekday().to_sunday_zero_offset()).days());
        std::array::from_fn(|offset| sunday.saturating_add((offset as i64).days()))
    }
}

/// A day of meetings for the gallery's preview.
pub fn sample(now: &Zoned) -> Agenda {
    let quarter_hours_ahead = |quarters: i64| {
        const QUARTER: i64 = 15 * 60;
        let next = (now.timestamp().as_second() / QUARTER + quarters) * QUARTER;
        Timestamp::from_second(next).unwrap_or(now.timestamp())
    };
    let meeting =
        |title: &str, start: Timestamp, minutes: i64, link: bool, place: Option<&str>| Event {
            title: title.to_owned(),
            when: When::Timed {
                start,
                end: start + SignedDuration::from_mins(minutes),
            },
            location: place.map(str::to_owned),
            color: Some(if link { "#5B5FEF" } else { "#2FB36B" }.to_owned()),
            meeting_url: link.then(|| "https://meet.google.com".to_owned()),
        };
    Agenda::Events(vec![
        Event {
            title: "Do not forget the weekly pill".to_owned(),
            when: When::AllDay(now.date()),
            location: None,
            color: None,
            meeting_url: None,
        },
        meeting(
            "Meeting with developers about system design and its problems.",
            quarter_hours_ahead(2),
            45,
            true,
            None,
        ),
        meeting(
            "Interview with designers scheduled for the new marketing project.",
            quarter_hours_ahead(6),
            45,
            false,
            Some("Office"),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_read_in_hours_and_minutes() {
        assert_eq!(length(SignedDuration::from_mins(45)), "45 min");
        assert_eq!(length(SignedDuration::from_mins(60)), "1 hr");
        assert_eq!(length(SignedDuration::from_mins(90)), "1 hr 30 min");
    }

    #[test]
    fn the_week_runs_sunday_to_saturday_around_today() -> Result<(), jiff::Error> {
        let wednesday: Zoned = "2026-08-12T10:00:00+00:00[UTC]".parse()?;
        let summary = Summary::of(None, &wednesday);
        let week = summary.week();
        assert_eq!(week[0].to_string(), "2026-08-09");
        assert_eq!(week[3], wednesday.date());
        assert_eq!(week[6].to_string(), "2026-08-15");
        Ok(())
    }

    #[test]
    fn a_signed_out_agenda_says_so_and_counts_nothing() -> Result<(), jiff::Error> {
        let now: Zoned = "2026-08-12T10:00:00+00:00[UTC]".parse()?;
        let summary = Summary::of(Some(&Agenda::SignedOut), &now);
        assert_eq!(summary.coming, Coming::SignedOut);
        assert_eq!(summary.events_today, 0);
        Ok(())
    }
}
