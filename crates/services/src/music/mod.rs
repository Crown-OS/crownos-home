//! What is playing on Spotify: from the desktop app over MPRIS when it runs
//! here, from the Web API when it plays elsewhere; with its cover and lyrics.

mod artwork;
mod feed;
mod lyrics;
mod mpris;
mod spotify;

use std::time::{Duration, Instant};

pub use artwork::fetch_cover;
pub use feed::follow;
pub use lyrics::{Lyrics, fetch_lyrics};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Track {
    pub title: String,
    pub artists: String,
    pub album: String,
    pub cover_url: Option<String>,
    pub duration: Duration,
}

/// Where the playback was read from, and so where commands for it go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Local,
    Web,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Playback {
    pub track: Track,
    pub progress: Duration,
    pub playing: bool,
    pub read_at: Instant,
    pub source: Source,
}

impl Playback {
    /// How far into the track playback is at `now`, counting on from the read.
    pub fn position(&self, now: Instant) -> Duration {
        if !self.playing {
            return self.progress;
        }
        (self.progress + now.saturating_duration_since(self.read_at)).min(self.track.duration)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Play,
    Pause,
    Next,
    Previous,
}

#[derive(Debug, thiserror::Error)]
pub enum MusicError {
    #[error("talking to the player")]
    Bus(#[from] zbus::Error),
    #[error(transparent)]
    Account(#[from] crownos_accounts::AccountError),
    #[error("talking to Spotify")]
    Request(#[from] ureq::Error),
    #[error("reading Spotify's answer")]
    Decode(#[from] serde_json::Error),
    #[error("the request was abandoned")]
    Abandoned,
}

/// The Spotify app's playback when it runs here, the account's otherwise.
pub async fn now_playing() -> Option<Playback> {
    match mpris::now_playing().await {
        Some(playback) => Some(playback),
        None => spotify::now_playing().await,
    }
}

pub async fn send(command: Transport, source: Source) {
    let sent = match source {
        Source::Local => mpris::send(command).await,
        Source::Web => spotify::send(command).await,
    };
    if let Err(e) = sent {
        log::warn!("music: {command:?}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playback(playing: bool) -> Playback {
        Playback {
            track: Track {
                title: "Song".to_owned(),
                artists: "Band".to_owned(),
                album: "Record".to_owned(),
                cover_url: None,
                duration: Duration::from_secs(200),
            },
            progress: Duration::from_secs(190),
            playing,
            read_at: Instant::now(),
            source: Source::Local,
        }
    }

    #[test]
    fn a_playing_track_moves_on_but_not_past_its_end() {
        let playing = playback(true);
        let later = playing.read_at + Duration::from_secs(5);
        assert_eq!(playing.position(later), Duration::from_secs(195));
        let much_later = playing.read_at + Duration::from_secs(60);
        assert_eq!(playing.position(much_later), Duration::from_secs(200));
    }

    #[test]
    fn a_paused_track_stays_put() {
        let paused = playback(false);
        let later = paused.read_at + Duration::from_secs(5);
        assert_eq!(paused.position(later), Duration::from_secs(190));
    }
}
