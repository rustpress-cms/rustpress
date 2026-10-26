//! Parser and serializer for PHP's `serialize()` format.
//!
//! WordPress stores options, post meta, widgets and menus as PHP-serialized
//! strings. Strings in this format are raw bytes (usually UTF-8, but not
//! guaranteed), and the length prefix counts bytes, so every string is kept as
//! `Vec<u8>` to stay byte-exact on a round trip.
//!
//! References (`R:n;` / `r:n;`) are kept as [`Value::Reference`] and
//! [`Value::ObjectRef`] with their slot number, so they serialize back
//! unchanged. Slots are numbered from 1 in parse order; array keys and object
//! property names are not counted, and `R:` itself does not take a slot (as in
//! PHP). `S:` strings (with `\xx` hex escapes) are decoded to [`Value::Str`]
//! and written back as plain `s:`, which is what PHP itself emits.
//!
//! Not supported yet: `C:` custom serialization and `E:` enums. They return
//! [`Error::Unsupported`].

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Int(i64),
    Str(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Vec<u8>),
    /// Ordered key/value pairs; PHP arrays keep insertion order.
    Array(Vec<(Key, Value)>),
    /// Property names keep their `\0*\0` / `\0Class\0` visibility prefix.
    /// Classes with `__serialize()` (e.g. `ArrayObject`) use integer keys.
    Object {
        class: Vec<u8>,
        props: Vec<(Key, Value)>,
    },
    /// `R:n;` – a PHP reference to the value in slot `n`.
    Reference(usize),
    /// `r:n;` – another handle to the object in slot `n`.
    ObjectRef(usize),
}

impl Value {
    /// The bytes of a string value.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A string value as UTF-8 (`None` for other types or invalid UTF-8).
    pub fn as_str(&self) -> Option<&str> {
        self.as_bytes().and_then(|b| std::str::from_utf8(b).ok())
    }

    /// Integers, and numeric strings the way PHP's `(int)` cast reads them.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Bool(b) => Some(*b as i64),
            Value::Float(f) => Some(*f as i64),
            Value::Str(_) => self.as_str()?.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[(Key, Value)]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Looks up a string key in an array.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_array()?.iter().find_map(|(k, v)| match k {
            Key::Str(s) if s == key.as_bytes() => Some(v),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnexpectedEof,
    /// Unexpected byte at the given offset.
    Unexpected { offset: usize, found: u8 },
    InvalidNumber { offset: usize },
    Unsupported { offset: usize, tag: u8 },
    /// `R:`/`r:` pointing at a slot that does not exist (yet).
    InvalidReference { offset: usize },
    TrailingData { offset: usize },
    /// Arrays and objects nested deeper than [`MAX_DEPTH`].
    TooDeep { offset: usize },
}

/// How deep arrays and objects may nest (PHP's `unserialize_max_depth` is 4096; WordPress data is far flatter, and
/// the parser recurses on the stack).
pub const MAX_DEPTH: usize = 128;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedEof => write!(f, "unexpected end of input"),
            Error::Unexpected { offset, found } => {
                write!(f, "unexpected byte {:?} at offset {offset}", *found as char)
            }
            Error::InvalidNumber { offset } => write!(f, "invalid number at offset {offset}"),
            Error::Unsupported { offset, tag } => {
                write!(f, "unsupported type tag {:?} at offset {offset}", *tag as char)
            }
            Error::InvalidReference { offset } => {
                write!(f, "invalid reference target at offset {offset}")
            }
            Error::TrailingData { offset } => write!(f, "trailing data at offset {offset}"),
            Error::TooDeep { offset } => write!(f, "nested too deep at offset {offset}"),
        }
    }
}

impl std::error::Error for Error {}

/// Parses a complete serialized value; trailing bytes are an error.
pub fn unserialize(input: &[u8]) -> Result<Value, Error> {
    let mut p = Parser { input, pos: 0, slots: 0, depth: 0 };
    let value = p.value()?;
    if p.pos != input.len() {
        return Err(Error::TrailingData { offset: p.pos });
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    /// Number of values seen so far that can be targets of `R:`/`r:`.
    slots: usize,
    /// Arrays and objects open around the current value.
    depth: usize,
}

impl Parser<'_> {
    fn enter(&mut self, offset: usize) -> Result<(), Error> {
        self.depth += 1;
        if self.depth > MAX_DEPTH { Err(Error::TooDeep { offset }) } else { Ok(()) }
    }

    fn peek(&self) -> Result<u8, Error> {
        self.input.get(self.pos).copied().ok_or(Error::UnexpectedEof)
    }

    fn expect(&mut self, byte: u8) -> Result<(), Error> {
        let found = self.peek()?;
        if found != byte {
            return Err(Error::Unexpected { offset: self.pos, found });
        }
        self.pos += 1;
        Ok(())
    }

    /// Reads bytes up to (not including) `end`, then consumes `end`.
    fn until(&mut self, end: u8) -> Result<&[u8], Error> {
        let rest = &self.input[self.pos..];
        let len = rest.iter().position(|&b| b == end).ok_or(Error::UnexpectedEof)?;
        self.pos += len + 1;
        Ok(&rest[..len])
    }

    fn int_until(&mut self, end: u8) -> Result<i64, Error> {
        let offset = self.pos;
        std::str::from_utf8(self.until(end)?)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::InvalidNumber { offset })
    }

    fn len_until(&mut self, end: u8) -> Result<usize, Error> {
        let offset = self.pos;
        usize::try_from(self.int_until(end)?).map_err(|_| Error::InvalidNumber { offset })
    }

    fn take(&mut self, len: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(len).ok_or(Error::UnexpectedEof)?;
        let bytes = self.input.get(self.pos..end).ok_or(Error::UnexpectedEof)?;
        self.pos = end;
        Ok(bytes)
    }

    /// `len:"bytes"` – the part shared by strings and class names.
    fn quoted(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.len_until(b':')?;
        self.expect(b'"')?;
        let bytes = self.take(len)?.to_vec();
        self.expect(b'"')?;
        Ok(bytes)
    }

    /// `len:"..."` for `S:`, where `\xx` is a hex-escaped byte and `len`
    /// counts the decoded bytes.
    fn quoted_escaped(&mut self) -> Result<Vec<u8>, Error> {
        let len = self.len_until(b':')?;
        self.expect(b'"')?;
        let mut bytes = Vec::with_capacity(len.min(4096));
        for _ in 0..len {
            let offset = self.pos;
            let byte = self.take(1)?[0];
            if byte != b'\\' {
                bytes.push(byte);
                continue;
            }
            let decoded = std::str::from_utf8(self.take(2)?)
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or(Error::InvalidNumber { offset })?;
            bytes.push(decoded);
        }
        self.expect(b'"')?;
        Ok(bytes)
    }

    /// Array keys and property names are values syntactically, but never
    /// take a reference slot.
    fn key(&mut self) -> Result<Key, Error> {
        let offset = self.pos;
        let slots = self.slots;
        let value = self.value();
        self.slots = slots;
        match value? {
            Value::Int(i) => Ok(Key::Int(i)),
            Value::Str(s) => Ok(Key::Str(s)),
            _ => Err(Error::Unexpected { offset, found: self.input[offset] }),
        }
    }

    fn reference(&mut self, tag_offset: usize) -> Result<usize, Error> {
        self.expect(b':')?;
        let target = self.len_until(b';')?;
        if target == 0 || target > self.slots {
            return Err(Error::InvalidReference { offset: tag_offset });
        }
        Ok(target)
    }

    fn value(&mut self) -> Result<Value, Error> {
        let tag_offset = self.pos;
        let tag = self.peek()?;
        self.pos += 1;
        if tag != b'R' {
            self.slots += 1;
        }
        match tag {
            b'N' => {
                self.expect(b';')?;
                Ok(Value::Null)
            }
            b'b' => {
                self.expect(b':')?;
                match self.int_until(b';')? {
                    0 => Ok(Value::Bool(false)),
                    1 => Ok(Value::Bool(true)),
                    _ => Err(Error::InvalidNumber { offset: tag_offset }),
                }
            }
            b'i' => {
                self.expect(b':')?;
                Ok(Value::Int(self.int_until(b';')?))
            }
            b'd' => {
                self.expect(b':')?;
                let offset = self.pos;
                let text = std::str::from_utf8(self.until(b';')?)
                    .map_err(|_| Error::InvalidNumber { offset })?;
                parse_float(text).map(Value::Float).ok_or(Error::InvalidNumber { offset })
            }
            b's' => {
                self.expect(b':')?;
                let bytes = self.quoted()?;
                self.expect(b';')?;
                Ok(Value::Str(bytes))
            }
            b'a' => {
                self.expect(b':')?;
                let count = self.len_until(b':')?;
                self.expect(b'{')?;
                self.enter(tag_offset)?;
                let mut items = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    let key = self.key()?;
                    items.push((key, self.value()?));
                }
                self.depth -= 1;
                self.expect(b'}')?;
                Ok(Value::Array(items))
            }
            b'O' => {
                self.expect(b':')?;
                let class = self.quoted()?;
                self.expect(b':')?;
                let count = self.len_until(b':')?;
                self.expect(b'{')?;
                self.enter(tag_offset)?;
                let mut props = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    let name = self.key()?;
                    props.push((name, self.value()?));
                }
                self.depth -= 1;
                self.expect(b'}')?;
                Ok(Value::Object { class, props })
            }
            b'S' => {
                self.expect(b':')?;
                let bytes = self.quoted_escaped()?;
                self.expect(b';')?;
                Ok(Value::Str(bytes))
            }
            b'R' => Ok(Value::Reference(self.reference(tag_offset)?)),
            b'r' => Ok(Value::ObjectRef(self.reference(tag_offset)?)),
            b'C' | b'E' => {
                Err(Error::Unsupported { offset: tag_offset, tag })
            }
            found => Err(Error::Unexpected { offset: tag_offset, found }),
        }
    }
}

fn parse_float(text: &str) -> Option<f64> {
    match text {
        "INF" => Some(f64::INFINITY),
        "-INF" => Some(f64::NEG_INFINITY),
        "NAN" => Some(f64::NAN),
        // PHP writes exponents as `1.0E+25`; Rust's parser accepts `E` and `+`.
        _ => text.parse().ok(),
    }
}

/// Serializes like PHP with `serialize_precision = -1` (the default since 7.1).
pub fn serialize(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_value(&mut out, value);
    out
}

fn write_str(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(format!("{}:\"", bytes.len()).as_bytes());
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\"");
}

fn write_key(out: &mut Vec<u8>, key: &Key) {
    match key {
        Key::Int(i) => out.extend_from_slice(format!("i:{i};").as_bytes()),
        Key::Str(s) => {
            out.extend_from_slice(b"s:");
            write_str(out, s);
            out.push(b';');
        }
    }
}

fn write_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => out.extend_from_slice(b"N;"),
        Value::Bool(b) => out.extend_from_slice(if *b { b"b:1;" } else { b"b:0;" }),
        Value::Int(i) => out.extend_from_slice(format!("i:{i};").as_bytes()),
        Value::Float(f) => out.extend_from_slice(format!("d:{};", format_float(*f)).as_bytes()),
        Value::Str(s) => {
            out.extend_from_slice(b"s:");
            write_str(out, s);
            out.push(b';');
        }
        Value::Reference(n) => out.extend_from_slice(format!("R:{n};").as_bytes()),
        Value::ObjectRef(n) => out.extend_from_slice(format!("r:{n};").as_bytes()),
        Value::Array(items) => {
            out.extend_from_slice(format!("a:{}:{{", items.len()).as_bytes());
            for (key, v) in items {
                write_key(out, key);
                write_value(out, v);
            }
            out.push(b'}');
        }
        Value::Object { class, props } => {
            out.extend_from_slice(b"O:");
            write_str(out, class);
            out.extend_from_slice(format!(":{}:{{", props.len()).as_bytes());
            for (name, v) in props {
                write_key(out, name);
                write_value(out, v);
            }
            out.push(b'}');
        }
    }
}

/// Shortest round-trip representation in PHP's notation: `0.1`, `1.0E+25`, `-0`.
fn format_float(f: f64) -> String {
    if f.is_nan() {
        return "NAN".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "INF" } else { "-INF" }.into();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0" } else { "0" }.into();
    }
    // `{:e}` yields the shortest digits and an exponent, e.g. `1.5e-7`.
    let sci = format!("{f:e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent form");
    let exp: i32 = exp.parse().expect("integer exponent");
    // PHP (php_gcvt with precision 17) switches to exponent notation outside
    // 1e-4 <= |x| < 1e17, e.g. `1.0E-5` but `0.0001`, `1.0E+17` but `10000000000000000`.
    if (-4..17).contains(&exp) {
        return format!("{f}");
    }
    let mantissa = if mantissa.contains('.') {
        mantissa.to_string()
    } else {
        format!("{mantissa}.0")
    };
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mantissa}E{sign}{}", exp.abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn s(text: &str) -> Value {
        Value::Str(text.as_bytes().to_vec())
    }

    fn k(text: &str) -> Key {
        Key::Str(text.as_bytes().to_vec())
    }

    #[test]
    fn scalars() {
        assert_eq!(unserialize(b"N;"), Ok(Value::Null));
        assert_eq!(unserialize(b"b:1;"), Ok(Value::Bool(true)));
        assert_eq!(unserialize(b"i:-42;"), Ok(Value::Int(-42)));
        assert_eq!(unserialize(b"d:0.1;"), Ok(Value::Float(0.1)));
        assert_eq!(unserialize(b"s:5:\"hello\";"), Ok(s("hello")));
    }

    #[test]
    fn string_length_counts_bytes_not_chars() {
        let value = unserialize("s:6:\"äöü\";".as_bytes()).unwrap();
        assert_eq!(value, s("äöü"));
        assert_eq!(serialize(&value), "s:6:\"äöü\";".as_bytes());
    }

    #[test]
    fn string_may_contain_quotes_and_semicolons() {
        let raw = b"s:8:\"a\";b:\"c;\";";
        assert_eq!(unserialize(raw), Ok(s("a\";b:\"c;")));
    }

    #[test]
    fn wordpress_style_array_round_trips() {
        // Shape of `active_plugins` / widget option values.
        let raw = b"a:2:{i:0;s:19:\"akismet/akismet.php\";s:3:\"foo\";a:1:{s:1:\"x\";b:0;}}";
        let value = unserialize(raw).unwrap();
        assert_eq!(
            value,
            Value::Array(vec![
                (Key::Int(0), s("akismet/akismet.php")),
                (k("foo"), Value::Array(vec![(k("x"), Value::Bool(false))])),
            ])
        );
        assert_eq!(serialize(&value), raw);
    }

    #[test]
    fn object_keeps_visibility_prefixes() {
        let raw = b"O:8:\"stdClass\":1:{s:4:\"\0*\0a\";i:1;}";
        let value = unserialize(raw).unwrap();
        assert_eq!(serialize(&value), raw);
    }

    #[test]
    fn float_formatting_matches_php() {
        assert_eq!(format_float(0.1), "0.1");
        assert_eq!(format_float(1.0), "1");
        assert_eq!(format_float(-0.0), "-0");
        assert_eq!(format_float(1e25), "1.0E+25");
        assert_eq!(format_float(1.5e-7), "1.5E-7");
        assert_eq!(format_float(100000.0), "100000");
        assert_eq!(format_float(1e15), "1000000000000000");
        assert_eq!(format_float(1e16), "10000000000000000");
        assert_eq!(format_float(1e17), "1.0E+17");
        assert_eq!(format_float(1.2345678901234568e17), "1.2345678901234568E+17");
        assert_eq!(format_float(0.0001), "0.0001");
        assert_eq!(format_float(1e-5), "1.0E-5");
        assert_eq!(format_float(f64::INFINITY), "INF");
    }

    #[test]
    fn references_keep_slot_numbers() {
        // Slot 1 is the array, slot 2 the string; keys are not counted.
        let raw = b"a:2:{i:0;s:1:\"x\";i:1;R:2;}";
        let value = unserialize(raw).unwrap();
        assert_eq!(
            value,
            Value::Array(vec![(Key::Int(0), s("x")), (Key::Int(1), Value::Reference(2))])
        );
        assert_eq!(serialize(&value), raw);
    }

    #[test]
    fn object_handles_round_trip() {
        let raw = b"a:2:{i:0;O:8:\"stdClass\":0:{}i:1;r:2;}";
        let value = unserialize(raw).unwrap();
        assert!(matches!(&value, Value::Array(items) if items[1].1 == Value::ObjectRef(2)));
        assert_eq!(serialize(&value), raw);
    }

    #[test]
    fn only_lowercase_r_takes_a_slot() {
        // Slots: array = 1, `i:7` = 2. `R:` takes no slot, so slot 3 is still free
        // afterwards; `r:` takes slot 3 itself.
        assert!(unserialize(b"a:3:{i:0;i:7;i:1;r:2;i:2;R:3;}").is_ok());
        assert!(unserialize(b"a:3:{i:0;i:7;i:1;R:2;i:2;R:3;}").is_err());
    }

    #[test]
    fn escaped_strings_decode_hex_bytes() {
        assert_eq!(unserialize(b"S:3:\"a\\62c\";"), Ok(s("abc")));
        assert_eq!(unserialize(b"S:1:\"\\ff\";"), Ok(Value::Str(vec![0xff])));
        assert!(unserialize(b"S:1:\"\\zz\";").is_err());
    }

    #[test]
    fn errors() {
        assert_eq!(unserialize(b"s:5:\"abc\";"), Err(Error::UnexpectedEof));
        assert_eq!(unserialize(b"i:1;x"), Err(Error::TrailingData { offset: 4 }));
        assert!(matches!(unserialize(b"C:1:\"A\":0:{}"), Err(Error::Unsupported { tag: b'C', .. })));
        assert_eq!(unserialize(b"R:1;"), Err(Error::InvalidReference { offset: 0 }));
        assert_eq!(unserialize(b"a:1:{i:0;R:0;}"), Err(Error::InvalidReference { offset: 9 }));
        assert!(matches!(unserialize(b"q:1;"), Err(Error::Unexpected { .. })));
        assert!(matches!(unserialize(b"b:2;"), Err(Error::InvalidNumber { .. })));
    }

    fn arb_value() -> impl Strategy<Value = super::Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::Int),
            any::<f64>().prop_filter("finite", |f| f.is_finite()).prop_map(Value::Float),
            proptest::collection::vec(any::<u8>(), 0..24).prop_map(Value::Str),
        ];
        leaf.prop_recursive(3, 24, 4, |inner| {
            proptest::collection::vec(
                (
                    prop_oneof![
                        any::<i64>().prop_map(Key::Int),
                        proptest::collection::vec(any::<u8>(), 0..8).prop_map(Key::Str),
                    ],
                    inner,
                ),
                0..4,
            )
            .prop_map(Value::Array)
        })
    }

    proptest! {
        #[test]
        fn round_trip(value in arb_value()) {
            let bytes = serialize(&value);
            prop_assert_eq!(unserialize(&bytes), Ok(value));
        }
    }
}

#[cfg(test)]
mod depth_tests {
    use super::*;

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let deep = "a:1:{i:0;".repeat(100_000) + "N;" + &"}".repeat(100_000);
        assert!(matches!(unserialize(deep.as_bytes()), Err(Error::TooDeep { .. })));
        let fine = "a:1:{i:0;".repeat(10) + "N;" + &"}".repeat(10);
        assert!(unserialize(fine.as_bytes()).is_ok());
    }
}
