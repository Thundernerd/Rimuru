//! A small key-value cache in SQLite, in the app's data folder.
//!
//! Entries are JSON, grouped by `kind` and keyed by `key`. An entry can
//! carry a `version` (e.g. a topic's reply count): a lookup with a different
//! version misses, which is how forum data is invalidated when new replies
//! arrive. Cache failures never fail a command; they just miss.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct Cache {
    conn: Mutex<Connection>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Cache {
    /// Opens the cache at `path`, or an in-memory one if that fails.
    pub fn open(path: &Path) -> Self {
        let conn = path
            .parent()
            .map(std::fs::create_dir_all)
            .transpose()
            .ok()
            .and_then(|_| Connection::open(path).ok())
            .unwrap_or_else(|| {
                log::warn!("couldn't open cache at {}; using memory", path.display());
                Connection::open_in_memory().expect("in-memory SQLite")
            });
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::init(Connection::open_in_memory().unwrap())
    }

    fn init(conn: Connection) -> Self {
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS entries (
                 kind      TEXT    NOT NULL,
                 key       TEXT    NOT NULL,
                 version   TEXT,
                 value     TEXT    NOT NULL,
                 stored_at INTEGER NOT NULL,
                 PRIMARY KEY (kind, key)
             );",
        );
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// The entry, if stored with the same `version` within `max_age`.
    pub fn get<T: DeserializeOwned>(
        &self,
        kind: &str,
        key: &str,
        version: Option<&str>,
        max_age: Duration,
    ) -> Option<T> {
        let conn = self.conn.lock().ok()?;
        let (stored_version, value, stored_at): (Option<String>, String, i64) = conn
            .query_row(
                "SELECT version, value, stored_at FROM entries WHERE kind = ?1 AND key = ?2",
                params![kind, key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .ok()??;
        let fresh = now() - stored_at <= max_age.as_secs() as i64;
        (fresh && stored_version.as_deref() == version)
            .then(|| serde_json::from_str(&value).ok())
            .flatten()
    }

    /// All entries of a kind stored within `max_age`, newest first.
    pub fn list<T: DeserializeOwned>(&self, kind: &str, max_age: Duration) -> Vec<T> {
        let Ok(conn) = self.conn.lock() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT value FROM entries WHERE kind = ?1 AND stored_at >= ?2 ORDER BY stored_at DESC",
        ) else {
            return Vec::new();
        };
        let oldest = now() - max_age.as_secs().min(i64::MAX as u64) as i64;
        stmt.query_map(params![kind, oldest], |row| row.get::<_, String>(0))
            .map(|rows| {
                rows.filter_map(|r| r.ok())
                    .filter_map(|json| serde_json::from_str(&json).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn put<T: Serialize>(&self, kind: &str, key: &str, version: Option<&str>, value: &T) {
        let Ok(json) = serde_json::to_string(value) else {
            return;
        };
        let Ok(conn) = self.conn.lock() else { return };
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO entries (kind, key, version, value, stored_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind, key, version, json, now()],
        ) {
            log::warn!("cache write failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: Duration = Duration::from_secs(86_400);

    #[test]
    fn round_trip_and_versions() {
        let cache = Cache::in_memory();
        cache.put("topic", "1", Some("34"), &vec![1, 2, 3]);
        assert_eq!(
            cache.get::<Vec<i32>>("topic", "1", Some("34"), DAY),
            Some(vec![1, 2, 3])
        );
        // New replies: a different version misses.
        assert_eq!(cache.get::<Vec<i32>>("topic", "1", Some("35"), DAY), None);
        assert_eq!(cache.get::<Vec<i32>>("topic", "1", None, DAY), None);
        assert_eq!(cache.get::<Vec<i32>>("other", "1", Some("34"), DAY), None);
        // Replacing an entry.
        cache.put("topic", "1", Some("35"), &vec![4]);
        assert_eq!(
            cache.get::<Vec<i32>>("topic", "1", Some("35"), DAY),
            Some(vec![4])
        );
    }

    #[test]
    fn lists_a_kind_newest_first() {
        let cache = Cache::in_memory();
        cache.put("visit", "1", None, &1);
        cache.put("visit", "2", None, &2);
        cache.put("other", "3", None, &3);
        cache
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE entries SET stored_at = stored_at - 10 WHERE key = '1'",
                [],
            )
            .unwrap();
        assert_eq!(cache.list::<i32>("visit", DAY), vec![2, 1]);
    }

    #[test]
    fn expires() {
        let cache = Cache::in_memory();
        cache.put("k", "a", None, &"x");
        cache
            .conn
            .lock()
            .unwrap()
            .execute("UPDATE entries SET stored_at = stored_at - 7200", [])
            .unwrap();
        assert_eq!(
            cache.get::<String>("k", "a", None, Duration::from_secs(3600)),
            None
        );
        assert_eq!(
            cache.get::<String>("k", "a", None, DAY).as_deref(),
            Some("x")
        );
    }
}
