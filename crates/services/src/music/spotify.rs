use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use crownos_accounts::{AccountError, Provider, access_token};
use serde::Deserialize;

use super::{MusicError, Playback, Source, Track, Transport};

const PLAYER: &str = "https://api.spotify.com/v1/me/player";
const NO_CONTENT: u16 = 204;

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(crownos_accounts::agent);

#[derive(Deserialize)]
struct Current {
    #[serde(default)]
    progress_ms: u64,
    #[serde(default)]
    is_playing: bool,
    item: Option<Item>,
}

#[derive(Deserialize)]
struct Item {
    name: String,
    duration_ms: u64,
    #[serde(default)]
    artists: Vec<Named>,
    album: Option<Album>,
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

#[derive(Deserialize)]
struct Album {
    name: String,
    #[serde(default)]
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Image {
    url: String,
}

fn bearer() -> Result<String, MusicError> {
    Ok(format!(
        "Bearer {}",
        access_token(&AGENT, Provider::Spotify)?
    ))
}

fn playback(current: Current) -> Option<Playback> {
    let item = current.item?;
    let (album, cover_url) = item.album.map_or_else(Default::default, |album| {
        (
            album.name,
            album.images.into_iter().next().map(|image| image.url),
        )
    });
    Some(Playback {
        track: Track {
            title: item.name,
            artists: item
                .artists
                .into_iter()
                .map(|artist| artist.name)
                .collect::<Vec<_>>()
                .join(", "),
            album,
            cover_url,
            duration: Duration::from_millis(item.duration_ms),
        },
        progress: Duration::from_millis(current.progress_ms),
        playing: current.is_playing,
        read_at: Instant::now(),
        source: Source::Web,
    })
}

fn read() -> Result<Option<Playback>, MusicError> {
    let mut response = AGENT
        .get(format!("{PLAYER}/currently-playing"))
        .header("Authorization", bearer()?)
        .call()?;
    if response.status().as_u16() == NO_CONTENT {
        return Ok(None);
    }
    let body = response.body_mut().read_to_string()?;
    Ok(playback(serde_json::from_str(&body)?))
}

/// The account's playback on any device, or nothing when signed out or idle.
pub async fn now_playing() -> Option<Playback> {
    let read = tokio::task::spawn_blocking(read).await;
    match read {
        Ok(Ok(playback)) => playback,
        Ok(Err(MusicError::Account(AccountError::SignedOut(_)))) => None,
        Ok(Err(e)) => {
            log::debug!("spotify: {e}");
            None
        }
        Err(_) => None,
    }
}

fn command(command: Transport) -> Result<(), MusicError> {
    let bearer = bearer()?;
    match command {
        Transport::Play => AGENT
            .put(format!("{PLAYER}/play"))
            .header("Authorization", bearer)
            .send_empty(),
        Transport::Pause => AGENT
            .put(format!("{PLAYER}/pause"))
            .header("Authorization", bearer)
            .send_empty(),
        Transport::Next => AGENT
            .post(format!("{PLAYER}/next"))
            .header("Authorization", bearer)
            .send_empty(),
        Transport::Previous => AGENT
            .post(format!("{PLAYER}/previous"))
            .header("Authorization", bearer)
            .send_empty(),
    }?;
    Ok(())
}

/// Playback control on the account; Spotify allows it to Premium accounts only.
pub async fn send(transport: Transport) -> Result<(), MusicError> {
    tokio::task::spawn_blocking(move || command(transport))
        .await
        .map_err(|_| MusicError::Abandoned)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_currently_playing_answer_reads_as_playback() -> Result<(), serde_json::Error> {
        let body = r#"{"progress_ms":192000,"is_playing":true,"currently_playing_type":"track",
            "item":{"name":"Disguise","duration_ms":349000,"artists":[{"name":"A"},{"name":"B"}],
            "album":{"name":"Hard","images":[{"url":"https://i.scdn.co/x","width":640}]}}}"#;
        let playback = playback(serde_json::from_str(body)?).expect("a track");
        assert_eq!(playback.track.artists, "A, B");
        assert_eq!(
            playback.track.cover_url.as_deref(),
            Some("https://i.scdn.co/x")
        );
        assert_eq!(playback.progress, Duration::from_secs(192));
        assert_eq!(playback.source, Source::Web);
        Ok(())
    }

    #[test]
    fn an_ad_break_reads_as_nothing_playing() -> Result<(), serde_json::Error> {
        let body =
            r#"{"progress_ms":0,"is_playing":true,"currently_playing_type":"ad","item":null}"#;
        assert!(playback(serde_json::from_str(body)?).is_none());
        Ok(())
    }
}
