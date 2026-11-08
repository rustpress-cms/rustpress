//! `WPForms\Vendor\TrueBV\Punycode` (true/punycode), which the e-mail field uses for its addresses. An error of the
//! library (a label out of bounds) is `None`.

const BASE: i64 = 36;
const TMIN: i64 = 1;
const TMAX: i64 = 26;
const SKEW: i64 = 38;
const DAMP: i64 = 700;
const INITIAL_BIAS: i64 = 72;
const INITIAL_N: i64 = 128;
const PREFIX: &str = "xn--";

const ENCODE_TABLE: &[u8; 36] = b"abcdefghijklmnopqrstuvwxyz0123456789";

fn decode_digit(c: Option<u8>) -> i64 {
    match c {
        Some(c @ b'a'..=b'z') => (c - b'a') as i64,
        Some(c @ b'0'..=b'9') => (c - b'0') as i64 + 26,
        // `$decodeTable[ $char ]` of a character outside the table is `null`.
        _ => 0,
    }
}

fn threshold(k: i64, bias: i64) -> i64 {
    if k <= bias + TMIN {
        TMIN
    } else if k >= bias + TMAX {
        TMAX
    } else {
        k - bias
    }
}

fn adapt(delta: i64, points: i64, first: bool) -> i64 {
    let mut delta = if first { delta / DAMP } else { delta / 2 };
    delta += delta / points;
    let mut k = 0;
    while delta > (BASE - TMIN) * TMAX / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + (BASE - TMIN + 1) * delta / (delta + SKEW)
}

/// `Punycode::encode()`: `mb_strtolower()` and every label encoded.
pub fn encode(input: &str) -> Option<String> {
    let input = input.to_lowercase();
    let mut parts = Vec::new();
    for part in input.split('.') {
        if part.is_empty() {
            return None;
        }
        parts.push(encode_part(part)?);
    }
    let output = parts.join(".");
    (output.len() <= 255).then_some(output)
}

/// `Punycode::encodePart()`.
fn encode_part(input: &str) -> Option<String> {
    let all: Vec<i64> = input.chars().map(|c| c as i64).collect();
    let basic: Vec<i64> = all.iter().copied().filter(|c| *c < 128).collect();
    let mut output: String = basic.iter().map(|c| *c as u8 as char).collect();
    if input == output {
        return Some(output);
    }
    let b = basic.len() as i64;
    let mut h = b;
    if b > 0 {
        output.push('-');
    }
    let mut non_basic: Vec<i64> = all.iter().copied().filter(|c| *c >= 128).collect();
    non_basic.sort_unstable();
    non_basic.dedup();
    let (mut n, mut bias, mut delta) = (INITIAL_N, INITIAL_BIAS, 0i64);
    let length = all.len() as i64;
    let mut i = 0;
    while h < length {
        let m = *non_basic.get(i)?;
        i += 1;
        delta += (m - n) * (h + 1);
        n = m;
        for &c in &all {
            if c < n || c < INITIAL_N {
                delta += 1;
            }
            if c == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = threshold(k, bias);
                    if q < t {
                        break;
                    }
                    let code = t + (q - t) % (BASE - t);
                    output.push(ENCODE_TABLE[code as usize] as char);
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                output.push(ENCODE_TABLE[q as usize] as char);
                bias = adapt(delta, h + 1, h == b);
                delta = 0;
                h += 1;
            }
        }
        delta += 1;
        n += 1;
    }
    let out = format!("{PREFIX}{output}");
    (!out.is_empty() && out.len() <= 63).then_some(out)
}

/// `Punycode::decode()`: `strtolower()` (ASCII) and the labels with `xn--` decoded.
pub fn decode(input: &str) -> Option<String> {
    let input = input.to_ascii_lowercase();
    let mut parts = Vec::new();
    for part in input.split('.') {
        if part.is_empty() || part.len() > 63 {
            return None;
        }
        match part.strip_prefix(PREFIX) {
            Some(rest) => parts.push(decode_part(rest)),
            None => parts.push(part.to_owned()),
        }
    }
    let output = parts.join(".");
    (output.len() <= 255).then_some(output)
}

/// `Punycode::decodePart()`.
fn decode_part(input: &str) -> String {
    let bytes = input.as_bytes();
    let (mut n, mut i, mut bias) = (INITIAL_N, 0i64, INITIAL_BIAS);
    let (mut output, mut pos): (Vec<char>, usize) = match input.rfind('-') {
        Some(p) => (input[..p].chars().collect(), p + 1),
        None => (Vec::new(), 0),
    };
    let mut output_length = output.len() as i64;
    // `strlen( $output )` counts bytes; the basic part is ASCII unless the input was not.
    if output.iter().any(|c| !c.is_ascii()) {
        output_length = input[..input.rfind('-').unwrap_or(0)].len() as i64;
    }
    while pos < bytes.len() {
        let old = i;
        let mut w = 1i64;
        let mut k = BASE;
        loop {
            let digit = decode_digit(bytes.get(pos).copied());
            pos += 1;
            i += digit * w;
            let t = threshold(k, bias);
            if digit < t {
                break;
            }
            w *= BASE - t;
            k += BASE;
        }
        output_length += 1;
        bias = adapt(i - old, output_length, old == 0);
        n += i / output_length;
        i %= output_length;
        let at = (i as usize).min(output.len());
        output.insert(at, char::from_u32(n as u32).unwrap_or('\u{fffd}'));
        i += 1;
    }
    output.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_like_the_library() {
        assert_eq!(encode("bücher").as_deref(), Some("xn--bcher-kva"));
        assert_eq!(encode("Example").as_deref(), Some("example"));
        assert_eq!(decode("xn--bcher-kva").as_deref(), Some("bücher"));
        assert_eq!(encode("münchen.de").as_deref(), Some("xn--mnchen-3ya.de"));
        assert_eq!(decode("XN--MNCHEN-3YA.DE").as_deref(), Some("münchen.de"));
        assert_eq!(encode("a..b"), None);
    }
}
