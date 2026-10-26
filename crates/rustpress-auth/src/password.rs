//! `wp_hash_password()` and `wp_check_password()`: the hash formats WordPress has used over time.
//!
//! - `$wp$2y$10$…`: bcrypt over `base64( hmac_sha384( password, "wp-sha384" ) )` (WordPress 6.8 and later).
//! - `$P$…` / `$H$…`: the portable phpass hash of older installations.
//! - `$2y$…`: plain bcrypt.
//! - 32 hex digits: the md5 of very old installations.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use sha2::Sha384;
use subtle::ConstantTimeEq;

/// The longest password WordPress hashes or checks.
const MAX_LENGTH: usize = 4096;
const BCRYPT_COST: u32 = 10;
const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// `base64_encode( hash_hmac( 'sha384', $password, 'wp-sha384', true ) )`.
fn prehash(password: &str) -> String {
    let mut mac = Hmac::<Sha384>::new_from_slice(b"wp-sha384").expect("hmac accepts any key length");
    mac.update(password.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

/// `wp_hash_password()`; `*` for a password that is too long, as in WordPress.
pub fn hash_password(password: &str) -> String {
    if password.len() > MAX_LENGTH {
        return "*".to_owned();
    }
    let hashed = bcrypt::hash(prehash(password.trim()), BCRYPT_COST).unwrap_or_default();
    // PHP writes `$2y$`; the algorithm is the same as Rust's `$2b$`.
    format!("$wp{}", hashed.replacen("$2b$", "$2y$", 1))
}

/// `wp_fast_hash()`: the keyed BLAKE2b hash WordPress 6.8 stores for security keys.
pub fn fast_hash(message: &str) -> String {
    use blake2::Blake2bMac;
    use blake2::digest::consts::U30;
    let mut mac = <Blake2bMac<U30> as hmac::digest::KeyInit>::new_from_slice(b"wp_fast_hash_6.8+").expect("a key of 17 bytes");
    hmac::digest::Update::update(&mut mac, message.as_bytes());
    let hashed = hmac::digest::FixedOutput::finalize_fixed(mac);
    format!("$generic${}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hashed))
}

/// `wp_verify_fast_hash()`: also accepts the phpass hashes of older versions.
pub fn verify_fast_hash(message: &str, hash: &str) -> bool {
    if !hash.starts_with("$generic$") {
        return phpass(message, hash).is_some_and(|computed| computed.as_bytes().ct_eq(hash.as_bytes()).into());
    }
    fast_hash(message).as_bytes().ct_eq(hash.as_bytes()).into()
}

/// `PasswordHash( 8, true )->HashPassword()`: the portable phpass hash of the cookie of a protected post.
pub fn hash_portable(password: &str) -> String {
    use rand::Rng;
    let mut random = [0u8; 6];
    rand::rng().fill(&mut random);
    let setting = format!("$P${}{}", ITOA64[13] as char, encode64(&random));
    phpass(password, &setting).unwrap_or_else(|| "*0".to_owned())
}

/// `wp_check_password()`.
pub fn check_password(password: &str, hash: &str) -> bool {
    if hash.len() <= 32 {
        return hash.as_bytes().ct_eq(format!("{:x}", Md5::digest(password.as_bytes())).as_bytes()).into();
    }
    if password.len() > MAX_LENGTH {
        return false;
    }
    if let Some(inner) = hash.strip_prefix("$wp") {
        return bcrypt::verify(prehash(password), inner).unwrap_or(false);
    }
    if hash.starts_with("$P$") {
        return phpass(password, hash).is_some_and(|computed| computed.as_bytes().ct_eq(hash.as_bytes()).into());
    }
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// `wp_password_needs_rehash()`: everything but the current format is rehashed after a sign-in.
pub fn needs_rehash(hash: &str) -> bool {
    match hash.strip_prefix("$wp") {
        Some(inner) => !inner.starts_with(&format!("$2y${BCRYPT_COST:02}$")),
        None => true,
    }
}

/// phpass `crypt_private()` for the portable `$P$` format: the hash a stored `setting` asks for.
fn phpass(password: &str, setting: &str) -> Option<String> {
    let bytes = setting.as_bytes();
    if !(setting.starts_with("$P$") || setting.starts_with("$H$")) || bytes.len() < 12 {
        return None;
    }
    let count_log2 = ITOA64.iter().position(|c| *c == bytes[3])?;
    if !(7..=30).contains(&count_log2) {
        return None;
    }
    let salt = &bytes[4..12];
    let mut hash = Md5::new().chain_update(salt).chain_update(password.as_bytes()).finalize();
    for _ in 0..(1u64 << count_log2) {
        hash = Md5::new().chain_update(hash).chain_update(password.as_bytes()).finalize();
    }
    Some(format!("{}{}", &setting[..12], encode64(&hash)))
}

/// phpass `encode64()`.
fn encode64(input: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    let count = input.len();
    loop {
        let mut value = u32::from(input[i]);
        i += 1;
        out.push(ITOA64[(value & 0x3f) as usize] as char);
        if i < count {
            value |= u32::from(input[i]) << 8;
        }
        out.push(ITOA64[((value >> 6) & 0x3f) as usize] as char);
        if i >= count {
            break;
        }
        i += 1;
        if i < count {
            value |= u32::from(input[i]) << 16;
        }
        out.push(ITOA64[((value >> 12) & 0x3f) as usize] as char);
        if i >= count {
            break;
        }
        i += 1;
        out.push(ITOA64[((value >> 18) & 0x3f) as usize] as char);
        if i >= count {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_format_round_trips() {
        let hash = hash_password("  geheim  ");
        assert!(hash.starts_with("$wp$2y$10$"));
        assert!(check_password("geheim", &hash));
        assert!(!check_password("anders", &hash));
        assert!(!needs_rehash(&hash));
    }

    #[test]
    fn phpass_known_vector() {
        // The documented phpass test vector: password "test12345".
        assert!(check_password("test12345", "$P$9IQRaTwmfeRo7ud9Fh4E2PdI0S3r.L0"));
        assert!(!check_password("test1234", "$P$9IQRaTwmfeRo7ud9Fh4E2PdI0S3r.L0"));
        assert!(needs_rehash("$P$9IQRaTwmfeRo7ud9Fh4E2PdI0S3r.L0"));
    }

    #[test]
    fn plain_bcrypt_and_md5_are_accepted() {
        let bcrypt_hash = bcrypt::hash("passwort", 4).unwrap();
        assert!(check_password("passwort", &bcrypt_hash));
        assert!(check_password("passwort", &format!("{:x}", Md5::digest(b"passwort"))));
        assert!(!check_password("falsch", &format!("{:x}", Md5::digest(b"passwort"))));
    }

    #[test]
    fn overlong_passwords_never_match() {
        let long = "x".repeat(MAX_LENGTH + 1);
        assert_eq!(hash_password(&long), "*");
        assert!(!check_password(&long, "$wp$2y$10$abcdefghijklmnopqrstuuabcdefghijklmnopqrstuvwxyz01234"));
    }
}

#[cfg(test)]
mod fast_hash_tests {
    use super::*;

    #[test]
    fn fast_hash_has_the_shape_of_wordpress() {
        let hash = fast_hash("abcdefghijklmnopqrst");
        assert!(hash.starts_with("$generic$"));
        // 30 bytes in base64 without padding: 40 characters.
        assert_eq!(hash.len(), "$generic$".len() + 40);
        // The value libsodium gives (`sodium_crypto_generichash( $key, 'wp_fast_hash_6.8+', 30 )`).
        assert_eq!(hash, "$generic$1BfirmYI698KI6-zrkeF0CJICMFgCxm3HV5z4zK9");
        assert!(verify_fast_hash("abcdefghijklmnopqrst", &hash));
        assert!(!verify_fast_hash("abcdefghijklmnopqrsu", &hash));
    }

    #[test]
    fn portable_hash_checks() {
        let hash = hash_portable("geheim");
        assert!(hash.starts_with("$P$B"));
        assert!(check_password("geheim", &hash));
        assert!(!check_password("anders", &hash));
    }
}
