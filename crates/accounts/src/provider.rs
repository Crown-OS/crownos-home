use serde::Deserialize;

use crate::Result;

/// Where the browser is sent back to once the user has signed in. Register it
/// with the provider exactly as written.
pub const REDIRECT_URI: &str = "http://127.0.0.1:8898/callback";
pub(crate) const REDIRECT_ADDRESS: &str = "127.0.0.1:8898";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Spotify,
    Google,
}

impl Provider {
    pub const ALL: [Self; 2] = [Self::Spotify, Self::Google];

    pub fn id(self) -> &'static str {
        match self {
            Self::Spotify => "spotify",
            Self::Google => "google",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Spotify => "Spotify",
            Self::Google => "Google",
        }
    }

    /// Google's desktop clients come with a secret to send along; Spotify's
    /// PKCE clients do not.
    pub fn needs_secret(self) -> bool {
        matches!(self, Self::Google)
    }

    /// Where the user registers the OAuth client this account signs in with.
    pub fn console_url(self) -> &'static str {
        match self {
            Self::Spotify => "https://developer.spotify.com/dashboard",
            Self::Google => "https://console.cloud.google.com/apis/credentials",
        }
    }

    pub(crate) fn authorize_url(self) -> &'static str {
        match self {
            Self::Spotify => "https://accounts.spotify.com/authorize",
            Self::Google => "https://accounts.google.com/o/oauth2/v2/auth",
        }
    }

    pub(crate) fn token_url(self) -> &'static str {
        match self {
            Self::Spotify => "https://accounts.spotify.com/api/token",
            Self::Google => "https://oauth2.googleapis.com/token",
        }
    }

    pub(crate) fn scopes(self) -> &'static str {
        match self {
            Self::Spotify => {
                "user-read-currently-playing user-read-playback-state user-modify-playback-state"
            }
            Self::Google => "https://www.googleapis.com/auth/calendar.readonly",
        }
    }

    /// Google only hands out a refresh token when asked for offline access.
    pub(crate) fn authorize_extras(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Spotify => &[],
            Self::Google => &[("access_type", "offline"), ("prompt", "consent")],
        }
    }

    /// The name the account is shown under once signed in.
    pub(crate) fn profile_name(self, agent: &ureq::Agent, token: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct SpotifyProfile {
            display_name: Option<String>,
            id: String,
        }
        #[derive(Deserialize)]
        struct GoogleCalendar {
            id: String,
        }
        let url = match self {
            Self::Spotify => "https://api.spotify.com/v1/me",
            Self::Google => "https://www.googleapis.com/calendar/v3/users/me/calendarList/primary",
        };
        let body = agent
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .call()?
            .body_mut()
            .read_to_string()?;
        Ok(match self {
            Self::Spotify => {
                let profile: SpotifyProfile = serde_json::from_str(&body)?;
                profile.display_name.unwrap_or(profile.id)
            }
            Self::Google => serde_json::from_str::<GoogleCalendar>(&body)?.id,
        })
    }
}
