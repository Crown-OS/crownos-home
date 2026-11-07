use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::sync::OnceCell;
use zbus::{
    Connection, Proxy,
    proxy::CacheProperties,
    zvariant::{OwnedValue, Value},
};

use super::{MusicError, Playback, Source, Track, Transport};

const SPOTIFY: &str = "org.mpris.MediaPlayer2.spotify";
const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";

static PLAYER_PROXY: OnceCell<Proxy<'static>> = OnceCell::const_new();

async fn player() -> Result<&'static Proxy<'static>, MusicError> {
    PLAYER_PROXY
        .get_or_try_init(|| async {
            let session = Connection::session().await?;
            Ok(zbus::proxy::Builder::new(&session)
                .destination(SPOTIFY)?
                .path(PATH)?
                .interface(PLAYER)?
                .cache_properties(CacheProperties::No)
                .build()
                .await?)
        })
        .await
}

fn text(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    match &**metadata.get(key)? {
        Value::Str(text) => Some(text.to_string()),
        _ => None,
    }
}

fn texts(metadata: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    let Some(Value::Array(array)) = metadata.get(key).map(|value| &**value) else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|item| match item {
            Value::Str(text) => Some(text.to_string()),
            _ => None,
        })
        .collect()
}

/// `mpris:length` and `Position` are microseconds, signed by the spec and
/// unsigned from some players.
fn microseconds(value: &Value<'_>) -> Duration {
    let micros = match value {
        Value::I64(micros) => u64::try_from(*micros).unwrap_or(0),
        Value::U64(micros) => *micros,
        _ => 0,
    };
    Duration::from_micros(micros)
}

fn track(metadata: &HashMap<String, OwnedValue>) -> Option<Track> {
    Some(Track {
        title: text(metadata, "xesam:title").filter(|title| !title.is_empty())?,
        artists: texts(metadata, "xesam:artist").join(", "),
        album: text(metadata, "xesam:album").unwrap_or_default(),
        cover_url: text(metadata, "mpris:artUrl"),
        duration: metadata
            .get("mpris:length")
            .map_or(Duration::ZERO, |length| microseconds(length)),
    })
}

async fn read() -> Result<Option<Playback>, MusicError> {
    let player = player().await?;
    let metadata: HashMap<String, OwnedValue> = player.get_property("Metadata").await?;
    let Some(track) = track(&metadata) else {
        return Ok(None);
    };
    let status: String = player.get_property("PlaybackStatus").await?;
    let position: OwnedValue = player.get_property("Position").await?;
    Ok(Some(Playback {
        track,
        progress: microseconds(&position),
        playing: status == "Playing",
        read_at: Instant::now(),
        source: Source::Local,
    }))
}

/// The Spotify app's playback, or nothing when it does not run here.
pub async fn now_playing() -> Option<Playback> {
    read()
        .await
        .map_err(|e| log::debug!("mpris: {e}"))
        .ok()
        .flatten()
}

pub async fn send(command: Transport) -> Result<(), MusicError> {
    let method = match command {
        Transport::Play => "Play",
        Transport::Pause => "Pause",
        Transport::Next => "Next",
        Transport::Previous => "Previous",
    };
    player().await?.call_method(method, &()).await?;
    Ok(())
}
