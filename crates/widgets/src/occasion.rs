//! Days that come round again — a weekly meeting, a birthday — or once.

use jiff::{
    Zoned,
    civil::{Date, Time, Weekday},
};
use serde::{Deserialize, Serialize};

/// An occasion as written in the settings: exactly one of `date`, `yearly`
/// (`"MM-DD"`) or `weekly` (a weekday), and optionally its hours.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Occasion {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yearly: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekly: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Once(Date),
    Yearly { month: i8, day: i8 },
    Weekly(Weekday),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub title: String,
    pub repeat: Repeat,
    pub start: Option<Time>,
    pub end: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OccasionError {
    #[error("`{0}` needs exactly one of `date`, `yearly` or `weekly`")]
    NoRepeat(String),
    #[error("`{title}`: `{value}` is not a {expected}")]
    Unreadable {
        title: String,
        value: String,
        expected: &'static str,
    },
}

impl Occasion {
    pub fn schedule(&self) -> Result<Schedule, OccasionError> {
        let unreadable = |value: &str, expected| OccasionError::Unreadable {
            title: self.title.clone(),
            value: value.to_owned(),
            expected,
        };
        let repeat = match (&self.date, &self.yearly, &self.weekly) {
            (Some(date), None, None) => Repeat::Once(
                date.parse()
                    .map_err(|_| unreadable(date, "date like 2026-08-18"))?,
            ),
            (None, Some(yearly), None) => {
                parse_month_day(yearly).ok_or_else(|| unreadable(yearly, "day like 08-18"))?
            }
            (None, None, Some(weekly)) => {
                Repeat::Weekly(parse_weekday(weekly).ok_or_else(|| unreadable(weekly, "weekday"))?)
            }
            _ => return Err(OccasionError::NoRepeat(self.title.clone())),
        };
        let time = |value: &Option<String>| {
            value
                .as_deref()
                .map(|text| {
                    text.parse::<Time>()
                        .map_err(|_| unreadable(text, "time like 17:00"))
                })
                .transpose()
        };
        Ok(Schedule {
            title: self.title.clone(),
            repeat,
            start: time(&self.start)?,
            end: time(&self.end)?,
        })
    }
}

fn parse_month_day(text: &str) -> Option<Repeat> {
    let (month, day) = text.split_once('-')?;
    let (month, day): (i8, i8) = (month.parse().ok()?, day.parse().ok()?);
    Date::new(2024, month, day).ok()?;
    Some(Repeat::Yearly { month, day })
}

fn parse_weekday(text: &str) -> Option<Weekday> {
    let lower = text.to_ascii_lowercase();
    [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ]
    .into_iter()
    .find(|weekday| {
        let name = format!("{weekday:?}").to_ascii_lowercase();
        lower.len() >= 3 && name.starts_with(&lower)
    })
}

impl Schedule {
    /// The first day on or after `day` the occasion falls on.
    pub fn on_or_after(&self, day: Date) -> Option<Date> {
        match self.repeat {
            Repeat::Once(date) => (date >= day).then_some(date),
            Repeat::Weekly(weekday) => {
                let ahead = (weekday.to_monday_zero_offset()
                    - day.weekday().to_monday_zero_offset())
                .rem_euclid(7);
                day.checked_add(jiff::Span::new().days(ahead)).ok()
            }
            Repeat::Yearly {
                month,
                day: of_month,
            } => [day.year(), day.year() + 1]
                .into_iter()
                .filter_map(|year| yearly_date(year, month, of_month))
                .find(|date| *date >= day),
        }
    }

    /// The next day the occasion is still to come on: today counts until its
    /// end time, or its start time when it has no end.
    pub fn next(&self, now: &Zoned) -> Option<Date> {
        let today = now.date();
        let over_today = self
            .end
            .or(self.start)
            .is_some_and(|until| now.time() >= until);
        let from = if over_today {
            today.tomorrow().ok()?
        } else {
            today
        };
        self.on_or_after(from)
    }

    pub fn hours(&self) -> Option<String> {
        let start = self.start?;
        let clock = |time: Time| format!("{:02}:{:02}", time.hour(), time.minute());
        Some(match self.end {
            Some(end) => format!("{} - {}", clock(start), clock(end)),
            None => clock(start),
        })
    }
}

/// `month`/`day` in `year`, a 29 February falling back to the 28th.
fn yearly_date(year: i16, month: i8, day: i8) -> Option<Date> {
    let first = Date::new(year, month, 1).ok()?;
    Date::new(year, month, day.min(first.days_in_month())).ok()
}

/// Whole days from `today` until `day`.
pub fn days_between(today: Date, day: Date) -> i64 {
    today
        .until(day)
        .map_or(0, |span| i64::from(span.get_days()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn occasion(fields: &str) -> Schedule {
        let source = format!("title = \"x\"\n{fields}");
        toml::from_str::<Occasion>(&source)
            .unwrap()
            .schedule()
            .unwrap()
    }

    fn at(day: Date, hour: i8) -> Zoned {
        day.at(hour, 0, 0, 0).in_tz("UTC").unwrap()
    }

    #[test]
    fn the_reference_board_counts_the_days_it_shows() {
        let today = date(2026, 5, 24);
        let birthday = occasion("yearly = \"06-02\"");
        let next = birthday.on_or_after(today).unwrap();
        assert_eq!(days_between(today, next), 9);
        let countdown = occasion("yearly = \"08-18\"").on_or_after(today).unwrap();
        assert_eq!(days_between(today, countdown), 86);
        assert_eq!(countdown.weekday(), Weekday::Tuesday);
    }

    #[test]
    fn a_weekly_occasion_falls_on_its_weekday() {
        let meeting = occasion("weekly = \"sunday\"\nstart = \"17:00\"\nend = \"18:00\"");
        assert_eq!(
            meeting.on_or_after(date(2026, 5, 24)),
            Some(date(2026, 5, 24))
        );
        assert_eq!(
            meeting.on_or_after(date(2026, 5, 25)),
            Some(date(2026, 5, 31))
        );
        assert_eq!(meeting.hours().as_deref(), Some("17:00 - 18:00"));
    }

    #[test]
    fn todays_occasion_moves_on_once_it_has_ended() {
        let meeting = occasion("weekly = \"sun\"\nstart = \"17:00\"\nend = \"18:00\"");
        let sunday = date(2026, 5, 24);
        assert_eq!(meeting.next(&at(sunday, 17)), Some(sunday));
        assert_eq!(meeting.next(&at(sunday, 19)), Some(date(2026, 5, 31)));
    }

    #[test]
    fn a_birthday_already_past_this_year_is_next_years() {
        let birthday = occasion("yearly = \"01-10\"");
        assert_eq!(
            birthday.on_or_after(date(2026, 5, 24)),
            Some(date(2027, 1, 10))
        );
    }

    #[test]
    fn a_leap_day_birthday_is_kept_on_the_28th_otherwise() {
        let leap = occasion("yearly = \"02-29\"");
        assert_eq!(leap.on_or_after(date(2027, 1, 1)), Some(date(2027, 2, 28)));
        assert_eq!(leap.on_or_after(date(2028, 1, 1)), Some(date(2028, 2, 29)));
    }

    #[test]
    fn a_past_one_off_has_no_next_day() {
        let once = occasion("date = \"2026-01-01\"");
        assert_eq!(once.on_or_after(date(2026, 5, 24)), None);
    }

    #[test]
    fn an_occasion_without_a_repeat_or_with_two_is_refused() {
        let none = Occasion {
            title: "x".into(),
            ..Occasion::default()
        };
        assert!(matches!(none.schedule(), Err(OccasionError::NoRepeat(_))));
        let both = Occasion {
            title: "x".into(),
            date: Some("2026-01-01".into()),
            weekly: Some("mon".into()),
            ..Occasion::default()
        };
        assert!(both.schedule().is_err());
    }

    #[test]
    fn unreadable_values_name_what_was_expected() {
        let bad = Occasion {
            title: "x".into(),
            yearly: Some("13-40".into()),
            ..Occasion::default()
        };
        assert!(matches!(
            bad.schedule(),
            Err(OccasionError::Unreadable { .. })
        ));
    }
}
