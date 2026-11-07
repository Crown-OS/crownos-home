//! The coming week of the user's Google calendars.

mod feed;
mod google;

use jiff::{Timestamp, civil::Date};

pub use feed::follow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    AllDay(Date),
    Timed { start: Timestamp, end: Timestamp },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub title: String,
    pub when: When,
    pub location: Option<String>,
    /// The calendar's colour, as `#rrggbb`.
    pub color: Option<String>,
    pub meeting_url: Option<String>,
}

impl Event {
    pub fn is_all_day(&self) -> bool {
        matches!(self.when, When::AllDay(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Agenda {
    /// No Google account is connected in Settings.
    SignedOut,
    /// The events of the coming week, soonest first.
    Events(Vec<Event>),
}

#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error(transparent)]
    Account(#[from] crownos_accounts::AccountError),
    #[error("talking to Google Calendar")]
    Request(#[from] ureq::Error),
    #[error("reading Google Calendar's answer")]
    Decode(#[from] serde_json::Error),
    #[error("reading a time")]
    Time(#[from] jiff::Error),
    #[error("the request was abandoned")]
    Abandoned,
}
