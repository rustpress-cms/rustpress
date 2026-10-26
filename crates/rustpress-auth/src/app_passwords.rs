//! Application passwords (`WP_Application_Passwords`): the list in the user meta `_application_passwords`,
//! and the check of a Basic authorization header against it.

use rand::Rng;
use rustpress_db::{Db, Writer};
use rustpress_phpser::{Key, Value};

use crate::password::{check_password, hash_password};

/// The user meta that holds the passwords.
pub const META_KEY: &str = "_application_passwords";

/// One entry of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPassword {
    pub uuid: String,
    pub app_id: String,
    pub name: String,
    /// `wp_hash_password()` of the password without its spaces.
    pub password: String,
    pub created: i64,
    pub last_used: Option<i64>,
    pub last_ip: Option<String>,
}

fn string(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// The list of a user from the serialized meta value; unreadable entries are skipped.
pub fn parse(raw: &str) -> Vec<AppPassword> {
    let Ok(Value::Array(items)) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Vec::new() };
    items
        .iter()
        .filter_map(|(_, item)| {
            let uuid = string(item.get("uuid"));
            if uuid.is_empty() {
                return None;
            }
            Some(AppPassword {
                uuid,
                app_id: string(item.get("app_id")),
                name: string(item.get("name")),
                password: string(item.get("password")),
                created: item.get("created").and_then(Value::as_i64).unwrap_or(0),
                last_used: item.get("last_used").and_then(Value::as_i64),
                last_ip: item.get("last_ip").and_then(Value::as_str).map(str::to_owned),
            })
        })
        .collect()
}

/// `serialize()` of the list.
pub fn serialize(list: &[AppPassword]) -> String {
    let field = |key: &str, value: Value| (Key::Str(key.as_bytes().to_vec()), value);
    let items = list
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let entry = vec![
                field("uuid", Value::Str(item.uuid.clone().into_bytes())),
                field("app_id", Value::Str(item.app_id.clone().into_bytes())),
                field("name", Value::Str(item.name.clone().into_bytes())),
                field("password", Value::Str(item.password.clone().into_bytes())),
                field("created", Value::Int(item.created)),
                field("last_used", item.last_used.map_or(Value::Null, Value::Int)),
                field("last_ip", item.last_ip.as_ref().map_or(Value::Null, |ip| Value::Str(ip.clone().into_bytes()))),
            ];
            (Key::Int(index as i64), Value::Array(entry))
        })
        .collect();
    String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(items))).into_owned()
}

/// `wp_generate_uuid4()`.
pub fn new_uuid() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

/// A fresh password: `wp_generate_password( 24, false )`.
pub fn new_password() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..24).map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char).collect()
}

/// The password as the user sees it: groups of four, separated by spaces (`chunk_split()`, trimmed).
pub fn display(password: &str) -> String {
    password.as_bytes().chunks(4).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join(" ")
}

/// The list of a user.
pub async fn list(db: &Db, user_id: u64) -> Vec<AppPassword> {
    db.user_meta(user_id, META_KEY).await.ok().flatten().map(|raw| parse(&raw)).unwrap_or_default()
}

/// Writes the list back.
pub async fn save(writer: &Writer, user_id: u64, list: &[AppPassword]) -> Result<(), rustpress_db::Error> {
    writer.update_user_meta(user_id, META_KEY, &serialize(list)).await
}

/// `wp_authenticate_application_password()`: the user and the entry a login and password belong to.
///
/// `Err(None)`: no application password is in use on the site, the request stays anonymous. `Err(Some((code,
/// message)))`: the error WordPress answers with (401).
pub async fn authenticate(db: &Db, login: &str, password: &str) -> Result<(u64, String), Option<(&'static str, &'static str)>> {
    // `WP_Application_Passwords::is_in_use()`
    let in_use = db.option("using_application_passwords").await.ok().flatten().is_some_and(|v| !v.is_empty() && v != "0");
    if !in_use {
        return Err(None);
    }
    let user = match db.user_by_login(login).await.ok().flatten() {
        Some(user) => Some(user),
        None if login.contains('@') => db.user_by_email(login).await.ok().flatten(),
        None => None,
    };
    let Some(user) = user else {
        return Err(Some(if login.contains('@') {
            ("invalid_email", "<strong>Error:</strong> Unknown email address. Check again or try your username.")
        } else {
            ("invalid_username", "<strong>Error:</strong> Unknown username. Check again or try your email address.")
        }));
    };
    // Everything that is not a letter or a digit goes (spaces and dashes of the displayed form).
    let plain: String = password.chars().filter(char::is_ascii_alphanumeric).collect();
    list(db, user.id)
        .await
        .into_iter()
        .find(|item| check_password(&plain, &item.password))
        .map(|item| (user.id, item.uuid))
        .ok_or(Some(("incorrect_password", "The provided password is an invalid application password.")))
}

/// `WP_Application_Passwords::record_application_password_usage()`.
pub async fn record_usage(db: &Db, writer: &Writer, user_id: u64, uuid: &str, ip: Option<&str>, now: i64) {
    let mut passwords = list(db, user_id).await;
    let Some(item) = passwords.iter_mut().find(|p| p.uuid == uuid) else { return };
    // Only once a day (`last_used` is not updated for each request).
    if item.last_used.is_some_and(|last| last + 86_400 > now) {
        return;
    }
    item.last_used = Some(now);
    item.last_ip = ip.map(str::to_owned);
    let _ = save(writer, user_id, &passwords).await;
}

/// Hashes a fresh password for storing.
pub fn hash(password: &str) -> String {
    hash_password(password)
}
