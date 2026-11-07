use std::{sync::LazyLock, time::Duration};

use crownhome_util::uri::encode_component;
use serde::Deserialize;

use super::Track;

const LRCLIB: &str = "https://lrclib.net/api/get";
const USER_AGENT: &str = "crownos-home (https://github.com/crown-os/crownos-home)";
const TIMEOUT: Duration = Duration::from_secs(10);

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
});

/// Lines of a song, each with the moment it is sung.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lyrics {
    lines: Vec<(Duration, String)>,
}

impl Lyrics {
    /// The line before the one sung at `at`, that line, and the one after.
    pub fn around(&self, at: Duration) -> [Option<&str>; 3] {
        let sung = self.lines.partition_point(|(start, _)| *start <= at);
        let line = |index: Option<usize>| {
            index
                .and_then(|index| self.lines.get(index))
                .map(|(_, text)| text.as_str())
        };
        [
            line(sung.checked_sub(2)),
            line(sung.checked_sub(1)),
            line(Some(sung)),
        ]
    }

    /// LRC text, one `[mm:ss.xx] words` line per sung line.
    pub fn parse(synced: &str) -> Self {
        let mut lines: Vec<_> = synced.lines().filter_map(parse_line).collect();
        lines.sort_by_key(|(start, _)| *start);
        Self { lines }
    }
}

/// `[01:02.50] words` as the moment and the words.
fn parse_line(line: &str) -> Option<(Duration, String)> {
    let (stamp, text) = line.strip_prefix('[')?.split_once(']')?;
    let (minutes, seconds) = stamp.split_once(':')?;
    let seconds = minutes.parse::<f64>().ok()? * 60.0 + seconds.parse::<f64>().ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| (Duration::from_secs_f64(seconds), text.to_owned()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
    synced_lyrics: Option<String>,
}

fn lookup(track: &Track) -> Result<Option<Lyrics>, ureq::Error> {
    let url = format!(
        "{LRCLIB}?track_name={}&artist_name={}&album_name={}&duration={}",
        encode_component(&track.title),
        encode_component(&track.artists),
        encode_component(&track.album),
        track.duration.as_secs()
    );
    let body = match AGENT.get(url).call() {
        Ok(mut response) => response.body_mut().read_to_string()?,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(serde_json::from_str::<Found>(&body)
        .ok()
        .and_then(|found| found.synced_lyrics)
        .map(|synced| Lyrics::parse(&synced))
        .filter(|lyrics| !lyrics.lines.is_empty()))
}

/// The synced lyrics LRCLIB has for `track`, if any.
pub async fn fetch_lyrics(track: Track) -> Option<Lyrics> {
    tokio::task::spawn_blocking(move || lookup(&track))
        .await
        .ok()?
        .map_err(|e| log::debug!("lyrics: {e}"))
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synced_lines_are_read_in_order_and_blank_ones_dropped() {
        let lyrics = Lyrics::parse("[00:12.50] second\n[00:03.00] first\n[00:20.00]\nnot a line");
        assert_eq!(
            lyrics.lines,
            [
                (Duration::from_secs(3), "first".to_owned()),
                (Duration::from_millis(12_500), "second".to_owned()),
            ]
        );
    }

    #[test]
    fn the_sung_line_sits_between_its_neighbours() {
        let lyrics = Lyrics::parse("[00:01.00] a\n[00:02.00] b\n[00:03.00] c");
        assert_eq!(
            lyrics.around(Duration::from_millis(2500)),
            [Some("a"), Some("b"), Some("c")]
        );
        assert_eq!(lyrics.around(Duration::ZERO), [None, None, Some("a")]);
        assert_eq!(
            lyrics.around(Duration::from_secs(9)),
            [Some("b"), Some("c"), None]
        );
    }
}
