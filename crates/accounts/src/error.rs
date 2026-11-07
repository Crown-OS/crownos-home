use std::path::PathBuf;

pub type Result<T, E = AccountError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("no configuration directory: $XDG_CONFIG_HOME and $HOME are both unset")]
    NoConfigDirectory,

    #[error("{0} is not signed in")]
    SignedOut(&'static str),

    #[error("{0} needs a client ID before signing in")]
    NoClient(&'static str),

    #[error("the sign-in was declined: {0}")]
    Declined(String),

    #[error("the sign-in was not finished in time")]
    TimedOut,

    #[error("the browser came back from a different sign-in")]
    StateMismatch,

    #[error("listening for the browser on {address}")]
    Listen {
        address: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("reading {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("writing {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("parsing {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("encoding the account")]
    Encode(#[from] toml::ser::Error),

    #[error("talking to the provider")]
    Request(#[from] ureq::Error),

    #[error("reading the provider's answer")]
    Decode(#[from] serde_json::Error),

    #[error("talking to the browser")]
    Browser(#[source] std::io::Error),

    #[error("no randomness for the sign-in")]
    Random(#[from] getrandom::Error),
}
