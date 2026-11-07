use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{AccountError, Provider, Result, util::secret_file};

/// The OAuth client the user registered with the provider.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub secret: String,
}

/// What a finished sign-in earned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Seconds since the Unix epoch.
    pub expires_at: u64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    #[serde(default)]
    pub client: Client,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Session>,
}

impl Account {
    /// `$XDG_CONFIG_HOME/crownos/accounts/<provider>.toml`.
    pub fn path(provider: Provider) -> Result<PathBuf> {
        dirs::config_dir()
            .map(|config| {
                config
                    .join("crownos/accounts")
                    .join(provider.id())
                    .with_extension("toml")
            })
            .ok_or(AccountError::NoConfigDirectory)
    }

    /// The saved account, or an empty one when there is none yet.
    pub fn load(provider: Provider) -> Result<Self> {
        let path = Self::path(provider)?;
        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                toml::from_str(&contents).map_err(|source| AccountError::Parse { path, source })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(AccountError::Read { path, source }),
        }
    }

    pub fn save(&self, provider: Provider) -> Result<()> {
        secret_file::write(&Self::path(provider)?, &toml::to_string_pretty(self)?)
    }

    pub fn is_signed_in(&self) -> bool {
        self.session.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_account_round_trips_without_an_empty_secret() {
        let account = Account {
            client: Client {
                id: "abc".to_owned(),
                secret: String::new(),
            },
            session: Some(Session {
                access_token: "a".to_owned(),
                refresh_token: "r".to_owned(),
                expires_at: 10,
                name: "me".to_owned(),
            }),
        };
        let encoded = toml::to_string_pretty(&account).expect("encodes");
        assert!(!encoded.contains("secret"));
        assert_eq!(
            toml::from_str::<Account>(&encoded).expect("parses"),
            account
        );
    }
}
