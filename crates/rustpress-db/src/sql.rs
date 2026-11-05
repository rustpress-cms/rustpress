//! A query builder that sends its values either as bound parameters or, in a process that answers one request (CGI),
//! written into the statement.
//!
//! A bound parameter needs a prepared statement, and preparing is a round trip of its own: a CGI process starts with
//! an empty statement cache, so a page of ~35 queries cost ~60 round trips to the database server, which on shared
//! hosting is another machine. Written into the statement, a query goes out over the text protocol in one round trip,
//! as `wpdb` sends every query. Values are never spliced in as text: numbers are written as digits and strings as
//! hexadecimal literals with the connection's character set (`_utf8mb4 X'…'`), which cannot end the literal whatever
//! they contain and do not depend on `NO_BACKSLASH_ESCAPES`; like a parameter, such a literal takes the collation of
//! the column it is compared with.

use std::fmt::Write as _;

use sqlx::mysql::{MySqlArguments, MySqlPool, MySqlRow};
use sqlx::{Arguments, AssertSqlSafe, FromRow, Row};

use crate::Result;

/// A value for a statement.
#[derive(Debug, Clone)]
pub(crate) enum Arg {
    Int(i64),
    UInt(u64),
    Str(String),
}

impl From<u64> for Arg {
    fn from(v: u64) -> Self {
        Arg::UInt(v)
    }
}
impl From<i64> for Arg {
    fn from(v: i64) -> Self {
        Arg::Int(v)
    }
}
impl From<u32> for Arg {
    fn from(v: u32) -> Self {
        Arg::UInt(v.into())
    }
}
impl From<i32> for Arg {
    fn from(v: i32) -> Self {
        Arg::Int(v.into())
    }
}
impl From<usize> for Arg {
    fn from(v: usize) -> Self {
        Arg::UInt(v as u64)
    }
}
impl From<String> for Arg {
    fn from(v: String) -> Self {
        Arg::Str(v)
    }
}
impl From<&String> for Arg {
    fn from(v: &String) -> Self {
        Arg::Str(v.clone())
    }
}
impl From<&str> for Arg {
    fn from(v: &str) -> Self {
        Arg::Str(v.to_owned())
    }
}

/// The SQL text of a value written into the statement.
fn literal(arg: &Arg, out: &mut String) {
    match arg {
        Arg::Int(v) => {
            let _ = write!(out, "{v}");
        }
        Arg::UInt(v) => {
            let _ = write!(out, "{v}");
        }
        Arg::Str(s) if s.is_empty() => out.push_str("''"),
        Arg::Str(s) => {
            out.push_str("_utf8mb4 X'");
            for b in s.bytes() {
                let _ = write!(out, "{b:02X}");
            }
            out.push('\'');
        }
    }
}

/// A statement under construction, with the API of `sqlx::QueryBuilder` that this crate uses.
pub(crate) struct Sql {
    text: String,
    args: Vec<Arg>,
    inline: bool,
}

impl Sql {
    pub(crate) fn new(init: impl std::fmt::Display) -> Self {
        Self { text: init.to_string(), args: Vec::new(), inline: crate::one_request() }
    }

    pub(crate) fn push(&mut self, sql: impl std::fmt::Display) -> &mut Self {
        let _ = write!(self.text, "{sql}");
        self
    }

    pub(crate) fn push_bind(&mut self, value: impl Into<Arg>) -> &mut Self {
        let value = value.into();
        if self.inline {
            literal(&value, &mut self.text);
        } else {
            self.text.push('?');
            self.args.push(value);
        }
        self
    }

    /// `push_bind` for each value, separated by `", "`.
    pub(crate) fn push_list<V: Into<Arg>>(&mut self, values: impl IntoIterator<Item = V>) -> &mut Self {
        for (i, value) in values.into_iter().enumerate() {
            if i > 0 {
                self.text.push_str(", ");
            }
            self.push_bind(value);
        }
        self
    }

    /// Values or SQL with `separator` between them (`QueryBuilder::separated`).
    pub(crate) fn separated(&mut self, separator: &'static str) -> Separated<'_> {
        Separated { sql: self, separator, first: true }
    }

    fn arguments(&self) -> MySqlArguments {
        let mut out = MySqlArguments::default();
        for arg in &self.args {
            // Encoding these types does not fail.
            let _ = match arg {
                Arg::Int(v) => out.add(*v),
                Arg::UInt(v) => out.add(*v),
                Arg::Str(v) => out.add(v.as_str()),
            };
        }
        out
    }

    async fn rows(self, pool: &MySqlPool) -> Result<Vec<MySqlRow>> {
        if self.args.is_empty() && self.inline {
            return Ok(sqlx::raw_sql(AssertSqlSafe(self.text)).fetch_all(pool).await?);
        }
        let arguments = self.arguments();
        Ok(sqlx::query_with(AssertSqlSafe(self.text), arguments).fetch_all(pool).await?)
    }

    pub(crate) async fn fetch_all<T: for<'r> FromRow<'r, MySqlRow>>(self, pool: &MySqlPool) -> Result<Vec<T>> {
        let rows = self.rows(pool).await?;
        Ok(rows.iter().map(T::from_row).collect::<std::result::Result<_, _>>()?)
    }

    pub(crate) async fn fetch_optional<T: for<'r> FromRow<'r, MySqlRow>>(self, pool: &MySqlPool) -> Result<Option<T>> {
        let rows = self.rows(pool).await?;
        Ok(rows.first().map(T::from_row).transpose()?)
    }

    /// The first column of every row.
    pub(crate) async fn fetch_column<T>(self, pool: &MySqlPool) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        let rows = self.rows(pool).await?;
        Ok(rows.iter().map(|row| row.try_get::<T, _>(0)).collect::<std::result::Result<_, _>>()?)
    }

    /// The first column of the first row.
    pub(crate) async fn fetch_scalar<T>(self, pool: &MySqlPool) -> Result<T>
    where
        T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        let rows = self.rows(pool).await?;
        let row = rows.first().ok_or(sqlx::Error::RowNotFound)?;
        Ok(row.try_get::<T, _>(0)?)
    }
}

/// See [`Sql::separated`].
pub(crate) struct Separated<'a> {
    sql: &'a mut Sql,
    separator: &'static str,
    first: bool,
}

impl Separated<'_> {
    fn separate(&mut self) {
        if !std::mem::take(&mut self.first) {
            self.sql.text.push_str(self.separator);
        }
    }

    pub(crate) fn push_bind(&mut self, value: impl Into<Arg>) -> &mut Self {
        self.separate();
        self.sql.push_bind(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_become_hex_literals() {
        let mut out = String::new();
        literal(&Arg::Str("it's \\ \"x\"".into()), &mut out);
        assert_eq!(out, "_utf8mb4 X'69742773205C20227822'");
        let mut out = String::new();
        literal(&Arg::Str(String::new()), &mut out);
        assert_eq!(out, "''");
        let mut out = String::new();
        literal(&Arg::Int(-5), &mut out);
        assert_eq!(out, "-5");
    }
}
