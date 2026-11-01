//! `$_POST` as PHP builds it from the fields of a form body, and the helpers Contact Form 7 reads it with.

/// A value of `$_POST`: a string, or an array of keys and values.
#[derive(Debug, Clone, PartialEq)]
pub enum Posted {
    Str(String),
    Map(Vec<(String, Posted)>),
}

impl Posted {
    fn from_php(value: rustpress_phpser::Value) -> Self {
        use rustpress_phpser::{Key, Value};
        match value {
            Value::Array(items) => Posted::Map(
                items
                    .into_iter()
                    .map(|(k, v)| {
                        let key = match k {
                            Key::Int(n) => n.to_string(),
                            Key::Str(s) => String::from_utf8_lossy(&s).into_owned(),
                        };
                        (key, Posted::from_php(v))
                    })
                    .collect(),
            ),
            Value::Str(bytes) => Posted::Str(String::from_utf8_lossy(&bytes).into_owned()),
            _ => Posted::Str(String::new()),
        }
    }

    /// `wpcf7_array_flatten()`.
    pub fn flatten(&self) -> Vec<String> {
        match self {
            Posted::Str(s) => vec![s.clone()],
            Posted::Map(items) => items.iter().flat_map(|(_, v)| v.flatten()).collect(),
        }
    }

    /// `map_deep( $value, $callback )`.
    pub fn map(&self, f: &dyn Fn(&str) -> String) -> Posted {
        match self {
            Posted::Str(s) => Posted::Str(f(s)),
            Posted::Map(items) => Posted::Map(items.iter().map(|(k, v)| (k.clone(), v.map(f))).collect()),
        }
    }

    /// `if ( $value )` / `! empty( $value )`.
    pub fn is_truthy(&self) -> bool {
        match self {
            Posted::Str(s) => !s.is_empty() && s != "0",
            Posted::Map(items) => !items.is_empty(),
        }
    }
}

/// `$_POST`: the top-level names in the order they first appear.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PostData(pub Vec<(String, Posted)>);

impl PostData {
    /// The fields of a form body. PHP turns dots and spaces of a top-level name into underscores.
    pub fn from_fields(fields: &[(String, String)]) -> Self {
        let renamed: Vec<(String, String)> = fields
            .iter()
            .map(|(name, value)| {
                let (base, rest) = name.find('[').map_or((name.as_str(), ""), |at| (&name[..at], &name[at..]));
                (format!("{}{rest}", base.replace(['.', ' '], "_")), value.clone())
            })
            .collect();
        let mut names: Vec<String> = Vec::new();
        for (name, _) in &renamed {
            let base = name.split('[').next().unwrap_or_default();
            if !base.is_empty() && !names.iter().any(|n| n == base) {
                names.push(base.to_owned());
            }
        }
        let mut out = Vec::new();
        for name in names {
            if let Some(value) = rustpress_core::native::form_value(&renamed, &name) {
                out.push((name, Posted::from_php(value)));
            }
        }
        Self(out)
    }

    pub fn get(&self, name: &str) -> Option<&Posted> {
        self.0.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// `wpcf7_superglobal_post( $key )`: the value cleaned like a submitted one, `''` when it was not sent.
    pub fn string(&self, name: &str) -> String {
        match self.get(name).map(clean) {
            Some(Posted::Str(s)) => s,
            Some(map) => map.flatten().into_iter().next().unwrap_or_default(),
            None => String::new(),
        }
    }
}

/// `wpcf7_strip_whitespaces( wp_kses_no_null( wp_scrub_utf8( $value ) ) )` on every string of the value.
pub fn clean(value: &Posted) -> Posted {
    value.map(&|s| super::formatter::strip_whitespaces(&kses_no_null(s), super::formatter::Side::Both).to_owned())
}

/// `wp_kses_no_null()`: no NUL bytes and no escaped zeros (`\0`).
pub fn kses_no_null(text: &str) -> String {
    static ESCAPED_ZERO: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"\\+0+").unwrap());
    ESCAPED_ZERO.replace_all(&text.replace('\0', ""), "").into_owned()
}

/// `wpcf7_flat_join( $input, array( 'separator' => $separator ) )`.
pub fn flat_join(values: &[String], separator: &str) -> String {
    values.iter().map(|v| super::formatter::php_trim(v)).collect::<Vec<_>>().join(separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_is_built_like_php() {
        let fields: Vec<(String, String)> = [("a", "1"), ("boxes[]", "x"), ("b.c", "2"), ("boxes[]", "y"), ("_wpcf7", "5")].iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        let post = PostData::from_fields(&fields);
        let names: Vec<&str> = post.0.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, ["a", "boxes", "b_c", "_wpcf7"]);
        assert_eq!(post.get("boxes").unwrap().flatten(), ["x", "y"]);
        assert_eq!(post.string("boxes"), "x");
        assert_eq!(post.string("missing"), "");
    }
}
