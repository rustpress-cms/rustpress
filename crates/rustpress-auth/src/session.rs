//! Sessions (`WP_User_Meta_Session_Tokens`): the sha256 of each live token is a key of the user's
//! `session_tokens` meta, which holds the expiration, the address and browser of the sign-in and its time.

use rand::Rng;
use rand::distr::Alphanumeric;
use rustpress_db::{Db, Writer};
use rustpress_phpser::{Key, Value};

use crate::cookie::sha256_hex;

/// `wp_generate_password( 43, false, false )`: letters and digits only.
pub fn generate_token() -> String {
    rand::rng().sample_iter(&Alphanumeric).take(43).map(char::from).collect()
}

fn bytes(text: &str) -> Value {
    Value::Str(text.as_bytes().to_vec())
}

fn key(text: &str) -> Key {
    Key::Str(text.as_bytes().to_vec())
}

/// The sessions that have not expired, as stored.
async fn live_sessions(db: &Db, user_id: u64, now: i64) -> Result<Vec<(Key, Value)>, rustpress_db::Error> {
    let Some(raw) = db.user_meta(user_id, "session_tokens").await? else { return Ok(Vec::new()) };
    let Ok(Value::Array(items)) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Ok(Vec::new()) };
    Ok(items.into_iter().filter(|(_, session)| session.get("expiration").and_then(Value::as_i64).is_some_and(|e| e >= now)).collect())
}

/// `update_sessions()`: the meta is deleted when no session is left.
async fn store(writer: &Writer, user_id: u64, sessions: Vec<(Key, Value)>) -> Result<(), rustpress_db::Error> {
    if sessions.is_empty() {
        writer.delete_user_meta(user_id, "session_tokens").await
    } else {
        let serialized = rustpress_phpser::serialize(&Value::Array(sessions));
        writer.update_user_meta(user_id, "session_tokens", &String::from_utf8_lossy(&serialized)).await
    }
}

/// `WP_Session_Tokens::create()`: a new token whose session expires at `expiration`.
pub async fn create_session(db: &Db, writer: &Writer, user_id: u64, expiration: i64, ip: Option<&str>, user_agent: Option<&str>, now: i64) -> Result<String, rustpress_db::Error> {
    let token = generate_token();
    let mut session: Vec<(Key, Value)> = vec![(key("expiration"), Value::Int(expiration))];
    if let Some(ip) = ip.filter(|i| !i.is_empty()) {
        session.push((key("ip"), bytes(ip)));
    }
    if let Some(agent) = user_agent.filter(|a| !a.is_empty()) {
        session.push((key("ua"), bytes(agent)));
    }
    session.push((key("login"), Value::Int(now)));
    let mut sessions = live_sessions(db, user_id, now).await?;
    let verifier = sha256_hex(&token);
    sessions.retain(|(k, _)| *k != key(&verifier));
    sessions.push((key(&verifier), Value::Array(session)));
    store(writer, user_id, sessions).await?;
    Ok(token)
}

/// `WP_Session_Tokens::update()`: stores the session of a known token (expiring at `expiration`).
pub async fn update_session(db: &Db, writer: &Writer, user_id: u64, token: &str, expiration: i64, now: i64) -> Result<(), rustpress_db::Error> {
    let verifier = sha256_hex(token);
    let mut sessions = live_sessions(db, user_id, now).await?;
    sessions.retain(|(k, _)| *k != key(&verifier));
    sessions.push((key(&verifier), Value::Array(vec![(key("expiration"), Value::Int(expiration)), (key("login"), Value::Int(now))])));
    store(writer, user_id, sessions).await
}

/// `WP_Session_Tokens::destroy()`: ends one session.
pub async fn destroy_session(db: &Db, writer: &Writer, user_id: u64, token: &str, now: i64) -> Result<(), rustpress_db::Error> {
    let verifier = sha256_hex(token);
    let mut sessions = live_sessions(db, user_id, now).await?;
    sessions.retain(|(k, _)| *k != key(&verifier));
    store(writer, user_id, sessions).await
}

/// `WP_Session_Tokens::destroy_others()`: every session but the one of `token` ends.
pub async fn destroy_other_sessions(db: &Db, writer: &Writer, user_id: u64, token: &str, now: i64) -> Result<(), rustpress_db::Error> {
    let verifier = sha256_hex(token);
    let mut sessions = live_sessions(db, user_id, now).await?;
    sessions.retain(|(k, _)| *k == key(&verifier));
    store(writer, user_id, sessions).await
}

/// `WP_Session_Tokens::destroy_all()`.
pub async fn destroy_all_sessions(writer: &Writer, user_id: u64) -> Result<(), rustpress_db::Error> {
    writer.delete_user_meta(user_id, "session_tokens").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_43_letters_and_digits() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 43);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(a, b);
    }
}
