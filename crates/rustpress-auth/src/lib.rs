//! Signing in and out, sessions and permissions, the way WordPress does them, so that the
//! cookies of a site keep working and the database needs no conversion.

pub mod caps;
pub mod cookie;
pub mod password;
pub mod salts;
pub mod session;
pub mod setcookie;

pub use caps::{CurrentUser, Roles};
pub use cookie::{Auth, ParsedCookie, Scheme, Session};
pub use salts::Salts;
