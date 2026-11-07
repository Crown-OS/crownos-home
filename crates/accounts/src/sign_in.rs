use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

use crate::{
    Account, AccountError, Client, Provider, Result,
    provider::{REDIRECT_ADDRESS, REDIRECT_URI},
    token,
    util::{pkce, query},
};

const POLL: Duration = Duration::from_millis(150);
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>CrownOS</title>\
<body style=\"font:16px system-ui;display:grid;place-items:center;height:90vh\">\
<p>Signed in. You can close this tab and return to Settings.</p>";

/// A sign-in in progress: the page the browser should open, and the loopback
/// listener waiting for the provider to send it back.
pub struct SignIn {
    provider: Provider,
    client: Client,
    verifier: String,
    state: String,
    url: String,
    listener: TcpListener,
}

/// What the browser brought back to the redirect URI.
enum Callback {
    Code(String),
    Declined(String),
    Elsewhere,
}

impl SignIn {
    pub fn start(provider: Provider, client: Client) -> Result<Self> {
        if client.id.trim().is_empty() {
            return Err(AccountError::NoClient(provider.title()));
        }
        let listener = TcpListener::bind(REDIRECT_ADDRESS)
            .and_then(|listener| listener.set_nonblocking(true).map(|()| listener))
            .map_err(|source| AccountError::Listen {
                address: REDIRECT_ADDRESS,
                source,
            })?;
        let verifier = pkce::random_token(48)?;
        let state = pkce::random_token(16)?;
        let challenge = pkce::challenge(&verifier);
        let mut pairs = vec![
            ("response_type", "code"),
            ("client_id", client.id.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("scope", provider.scopes()),
            ("state", state.as_str()),
            ("code_challenge_method", "S256"),
            ("code_challenge", challenge.as_str()),
        ];
        pairs.extend_from_slice(provider.authorize_extras());
        let url = format!("{}?{}", provider.authorize_url(), query::build(&pairs));
        Ok(Self {
            provider,
            client,
            verifier,
            state,
            url,
            listener,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Waits until `deadline` for the browser, trades the code it brings for
    /// tokens, and saves the signed-in account.
    pub fn finish(self, deadline: Instant) -> Result<Account> {
        let code = self.wait_for_code(deadline)?;
        let agent = token::agent();
        let grant = token::grant(
            &agent,
            self.provider,
            &self.client,
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", REDIRECT_URI),
                ("code_verifier", &self.verifier),
            ],
        )?;
        let name = self
            .provider
            .profile_name(&agent, grant.access_token())
            .unwrap_or_else(|e| {
                log::warn!("{}: reading the profile: {e}", self.provider.title());
                String::new()
            });
        let account = Account {
            client: self.client,
            session: Some(grant.into_session("", name)),
        };
        account.save(self.provider)?;
        Ok(account)
    }

    fn wait_for_code(&self, deadline: Instant) -> Result<String> {
        while Instant::now() < deadline {
            match self.listener.accept() {
                Ok((stream, _)) => match self.answer(stream)? {
                    Callback::Code(code) => return Ok(code),
                    Callback::Declined(reason) => return Err(AccountError::Declined(reason)),
                    Callback::Elsewhere => {}
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(POLL),
                Err(e) => return Err(AccountError::Browser(e)),
            }
        }
        Err(AccountError::TimedOut)
    }

    fn answer(&self, mut stream: TcpStream) -> Result<Callback> {
        stream
            .set_nonblocking(false)
            .and_then(|()| stream.set_read_timeout(Some(READ_TIMEOUT)))
            .map_err(AccountError::Browser)?;
        let mut request_line = String::new();
        BufReader::new(&stream)
            .read_line(&mut request_line)
            .map_err(AccountError::Browser)?;
        let target = request_line.split_whitespace().nth(1).unwrap_or_default();
        let Some(query) = target.strip_prefix("/callback?") else {
            respond(&mut stream, "404 Not Found", "")?;
            return Ok(Callback::Elsewhere);
        };
        respond(&mut stream, "200 OK", DONE_PAGE)?;
        if query::find(query, "state").as_deref() != Some(self.state.as_str()) {
            return Err(AccountError::StateMismatch);
        }
        Ok(
            match (query::find(query, "code"), query::find(query, "error")) {
                (Some(code), _) => Callback::Code(code),
                (None, reason) => Callback::Declined(reason.unwrap_or_default()),
            },
        )
    }
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .map_err(AccountError::Browser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sign_in_needs_a_client_id() {
        let started = SignIn::start(Provider::Spotify, Client::default());
        assert!(matches!(started, Err(AccountError::NoClient("Spotify"))));
    }
}
