//! What the installer learns about database settings before it writes them down: `wpdb::db_connect()`,
//! `wpdb::select()` and the check of the table prefix in `wp-admin/setup-config.php`.

use sqlx::{AssertSqlSafe, Connection, Executor, MySqlConnection};

/// The outcome of [probe].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// The server answers and the database can be used; with the server's version.
    Ready { version: String },
    /// No connection: the server is not reachable or refuses the user.
    NoConnection,
    /// Connected, but the database cannot be selected (missing or not allowed).
    NoDatabase,
    /// `SELECT <prefix>` is valid SQL: the prefix would be read as a value (`setup-config.php` refuses it).
    PrefixParses,
}

/// Connects to `server_url` (a `mysql://` URL without database), selects `database` and checks `prefix`.
pub async fn probe(server_url: &str, database: &str, prefix: &str) -> Probe {
    let Ok(mut connection) = MySqlConnection::connect(server_url).await else {
        return Probe::NoConnection;
    };
    let quoted = format!("`{}`", database.replace('`', "``"));
    if connection.execute(AssertSqlSafe(format!("USE {quoted}"))).await.is_err() {
        return Probe::NoDatabase;
    }
    if !prefix.is_empty() && connection.execute(AssertSqlSafe(format!("SELECT {prefix}"))).await.is_ok() {
        return Probe::PrefixParses;
    }
    let version: String = sqlx::query_scalar("SELECT VERSION()").fetch_one(&mut connection).await.unwrap_or_default();
    let _ = connection.close().await;
    Probe::Ready { version }
}
