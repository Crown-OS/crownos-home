use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::{Account, AccountError, Client, Provider, Result, Session};

const TIMEOUT: Duration = Duration::from_secs(15);
/// A token this close to running out is refreshed before it is handed out.
const EXPIRY_MARGIN: u64 = 60;

#[derive(Deserialize)]
pub(crate) struct Grant {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

impl Grant {
    /// The session this grant starts, keeping `previous_refresh` when the
    /// provider did not issue a new refresh token.
    pub(crate) fn into_session(self, previous_refresh: &str, name: String) -> Session {
        Session {
            access_token: self.access_token,
            refresh_token: self
                .refresh_token
                .unwrap_or_else(|| previous_refresh.to_owned()),
            expires_at: now() + self.expires_in,
            name,
        }
    }

    pub(crate) fn access_token(&self) -> &str {
        &self.access_token
    }
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Posts `form` to the provider's token endpoint, with the client's identity.
pub(crate) fn grant(
    agent: &ureq::Agent,
    provider: Provider,
    client: &Client,
    form: &[(&str, &str)],
) -> Result<Grant> {
    let identity = [
        ("client_id", client.id.as_str()),
        ("client_secret", client.secret.as_str()),
    ];
    let fields = form
        .iter()
        .chain(identity.iter().filter(|(_, value)| !value.is_empty()));
    let body = agent
        .post(provider.token_url())
        .send_form(fields.copied())?
        .body_mut()
        .read_to_string()?;
    Ok(serde_json::from_str(&body)?)
}

/// A token for `provider` that is good for at least another minute.
pub fn access_token(agent: &ureq::Agent, provider: Provider) -> Result<String> {
    let mut account = Account::load(provider)?;
    let session = account
        .session
        .as_ref()
        .ok_or(AccountError::SignedOut(provider.title()))?;
    if session.expires_at > now() + EXPIRY_MARGIN {
        return Ok(session.access_token.clone());
    }
    let refreshed = grant(
        agent,
        provider,
        &account.client,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", &session.refresh_token),
        ],
    )?
    .into_session(&session.refresh_token, session.name.clone());
    let token = refreshed.access_token.clone();
    account.session = Some(refreshed);
    account.save(provider)?;
    Ok(token)
}

/// Forgets the tokens and keeps the client, so signing in again is one click.
pub fn sign_out(provider: Provider) -> Result<()> {
    let mut account = Account::load(provider)?;
    if account.session.take().is_some() {
        account.save(provider)?;
    }
    Ok(())
}
