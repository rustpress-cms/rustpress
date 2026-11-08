//! `WPForms\Forms\Token`: the antispam token of a form (`data-token`) and the signed time it was printed at
//! (`wpforms[time_token]`), both made with the secret key of `WPForms\Helpers\Crypto::get_secret_key()`.

use hmac::{Hmac, Mac};

/// `DAY_IN_SECONDS`.
const DAY: i64 = 86_400;

/// `Token::get( $time, $form_data )`: `md5( gmdate( 'dmYzW', $time ) . "::{$form_id}" . $secret )`. `form_id` is
/// `$form_data['id']` as the form data holds it; empty leaves the suffix out.
pub fn get(time: i64, form_id: &str, secret: &[u8]) -> String {
    let date = chrono::DateTime::from_timestamp(time, 0).unwrap_or_default();
    // `z` is the day of the year from 0 without padding, `W` the ISO week with two digits.
    let mut data = format!("{}{}{}", date.format("%d%m%Y"), date.format("%j").to_string().parse::<u32>().unwrap_or(1) - 1, date.format("%V")).into_bytes();
    if !form_id.is_empty() && form_id != "0" {
        data.extend_from_slice(format!("::{form_id}").as_bytes());
    }
    data.extend_from_slice(secret);
    format!("{:x}", md5::compute(&data))
}

/// `Token::verify()`: one of the tokens of `get_valid_tokens()`, the days of five years before, today and 45
/// minutes after now.
pub fn verify(token: &str, form_id: &str, secret: &[u8], now: i64) -> bool {
    let mut times: Vec<i64> = (1..=5 * 365).map(|i| now - i * DAY).collect();
    times.push(now);
    times.push(now + 45 * 60);
    // The token changes once a day: every distinct date is checked once.
    let mut seen = std::collections::HashSet::new();
    times.into_iter().filter(|t| seen.insert(chrono::DateTime::from_timestamp(*t, 0).unwrap_or_default().date_naive())).any(|t| get(t, form_id, secret) == token)
}

/// `Token::sign_time()`: `hash_hmac( 'sha256', "{$time}|{$form_id}", $secret )`.
fn sign_time(time: i64, form_id: i64, secret: &[u8]) -> String {
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).expect("any key length");
    mac.update(format!("{time}|{form_id}").as_bytes());
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

/// `Token::get_time_token()`: `"{$time}.{$signature}"` with `absint( $form_data['id'] )`.
pub fn time_token(time: i64, form_id: i64, secret: &[u8]) -> String {
    format!("{time}.{}", sign_time(time, form_id, secret))
}

/// `Token::verify_time_token()`: the time the form was printed at, 0 when the token is not valid or lies ahead.
pub fn verify_time_token(token: &str, form_id: i64, secret: &[u8], now: i64) -> i64 {
    let parts: Vec<&str> = token.split('.').collect();
    let [time, signature] = parts[..] else { return 0 };
    if time.is_empty() || !time.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    let Ok(render) = time.parse::<i64>() else { return 0 };
    if sign_time(render, form_id, secret) != signature || render > now {
        return 0;
    }
    render
}

/// `Crypto::get_secret_key()`: the option `wpforms_crypto_secret_key`, base64 decoded.
pub fn secret_key(option: Option<&str>) -> Vec<u8> {
    use base64::Engine;
    let Some(option) = option else { return Vec::new() };
    // base64_decode() skips characters outside the alphabet.
    let clean: String = option.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/')).collect();
    base64::engine::general_purpose::STANDARD_NO_PAD.decode(clean.as_bytes()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_like_php() {
        // php -r 'echo gmdate("dmYzW", 1794167509);' prints 0811202631145.
        assert_eq!(get(1794167509, "", b""), format!("{:x}", md5::compute(b"0811202631145")));
        assert!(verify(&get(1794167509 - 3 * DAY, "5", b"k"), "5", b"k", 1794167509));
        assert!(!verify(&get(1794167509, "6", b"k"), "5", b"k", 1794167509));
        let t = time_token(100, 5, b"k");
        assert_eq!(verify_time_token(&t, 5, b"k", 200), 100);
        assert_eq!(verify_time_token(&t, 6, b"k", 200), 0);
        assert_eq!(verify_time_token(&t, 5, b"k", 50), 0);
    }
}
