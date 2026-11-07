//! Online accounts shared by CrownOS apps.
//!
//! Settings signs in with [`SignIn`]: an OAuth authorization-code flow with
//! PKCE, redirected to a loopback listener. The account, the OAuth client the
//! user registered and the tokens it earned are kept per [`Provider`] in a file
//! only the user can read, and [`access_token`] hands any app a token that is
//! still good, refreshing it first when it has run out.

mod error;
mod provider;
mod sign_in;
mod store;
mod token;
mod util;

pub use error::{AccountError, Result};
pub use provider::{Provider, REDIRECT_URI};
pub use sign_in::SignIn;
pub use store::{Account, Client, Session};
pub use token::{access_token, agent, sign_out};
