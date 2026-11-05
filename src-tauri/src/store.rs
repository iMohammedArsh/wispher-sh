//! Local SQLite storage: settings, history (with full-text search), dictionary,
//! snippets and learned suggestions. One connection behind a mutex; every write is
//! a single statement or transaction, so a crash never leaves half-written JSON.

use std::path::Path;

use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::AppResult;

const MIGRATIONS: &[&str] = &[
    // v1
    r#"
    CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);

    CREATE TABLE history (
        id          INTEGER PRIMARY KEY,
        created_at  INTEGER NOT NULL,
        text        TEXT    NOT NULL,
        raw         TEXT    NOT NULL DEFAULT '',
        app         TEXT    NOT NULL DEFAULT '',
        style       TEXT    NOT NULL DEFAULT '',
        delivery    TEXT    NOT NULL DEFAULT '',
        duration_ms INTEGER NOT NULL DEFAULT 0,
        words       INTEGER NOT NULL DEFAULT 0,
        pinned      INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX history_created ON history(created_at DESC);

    CREATE VIRTUAL TABLE history_fts USING fts5(text, content='history', content_rowid='id');
    CREATE TRIGGER history_ai AFTER INSERT ON history BEGIN
        INSERT INTO history_fts(rowid, text) VALUES (new.id, new.text);
    END;
    CREATE TRIGGER history_ad AFTER DELETE ON history BEGIN
        INSERT INTO history_fts(history_fts, rowid, text) VALUES ('delete', old.id, old.text);
    END;
    CREATE TRIGGER history_au AFTER UPDATE OF text ON history BEGIN
        INSERT INTO history_fts(history_fts, rowid, text) VALUES ('delete', old.id, old.text);
        INSERT INTO history_fts(rowid, text) VALUES (new.id, new.text);
    END;

    CREATE TABLE dictionary (
        id          INTEGER PRIMARY KEY,
        term        TEXT NOT NULL UNIQUE COLLATE NOCASE,
        replacement TEXT,
        source      TEXT NOT NULL DEFAULT 'user',
        created_at  INTEGER NOT NULL
    );

    CREATE TABLE snippets (
        id         INTEGER PRIMARY KEY,
        trigger    TEXT NOT NULL UNIQUE COLLATE NOCASE,
        expansion  TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );

    CREATE TABLE suggestions (
        id         INTEGER PRIMARY KEY,
        heard      TEXT NOT NULL,
        correct    TEXT NOT NULL,
        count      INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL,
        UNIQUE(heard, correct)
    );
    "#,
];

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn word_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryItem {
    pub id: i64,
    pub created_at: i64,
    pub text: String,
    pub app: String,
    pub style: String,
    pub delivery: String,
    pub duration_ms: i64,
    pub words: i64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NewHistory<'a> {
    pub text: &'a str,
    pub raw: &'a str,
    pub app: &'a str,
    pub style: &'a str,
    pub delivery: &'a str,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DictionaryEntry {
    pub id: i64,
    pub term: String,
    /// When set, `term` is what Whisper hears and this is what gets written.
    pub replacement: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snippet {
    pub id: i64,
    pub trigger: String,
    pub expansion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suggestion {
    pub id: i64,
    pub heard: String,
    pub correct: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Stats {
    pub total_words: i64,
    pub total_dictations: i64,
    pub words_today: i64,
    pub average_wpm: i64,
    /// Typing at ~40 wpm vs speaking at the measured rate.
    pub minutes_saved: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    /// (local date "YYYY-MM-DD", words) for the last 120 days with activity.
    pub activity: Vec<(String, i64)>,
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> AppResult<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            conn.execute_batch(&format!("BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;", index + 1))?;
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    // ---- key/value -------------------------------------------------------

    pub fn get_json<T: DeserializeOwned>(&self, key: &str) -> AppResult<Option<T>> {
        let conn = self.conn.lock();
        let value: Option<String> = conn
            .query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
            .optional()?;
        Ok(match value {
            Some(json) => Some(serde_json::from_str(&json)?),
            None => None,
        })
    }

    pub fn set_json<T: Serialize>(&self, key: &str, value: &T) -> AppResult<()> {
        let json = serde_json::to_string(value)?;
        self.conn.lock().execute(
            "INSERT INTO kv(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, json],
        )?;
        Ok(())
    }

    // ---- history ---------------------------------------------------------

    pub fn add_history(&self, item: &NewHistory) -> AppResult<i64> {
        self.add_history_at(item, now_ms())
    }

    pub fn add_history_at(&self, item: &NewHistory, created_at: i64) -> AppResult<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO history(created_at, text, raw, app, style, delivery, duration_ms, words)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                created_at,
                item.text,
                item.raw,
                item.app,
                item.style,
                item.delivery,
                item.duration_ms,
                word_count(item.text)
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Newest first. `query` uses SQLite full-text search (prefix match on each word).
    pub fn history(&self, query: &str, limit: i64) -> AppResult<Vec<HistoryItem>> {
        let conn = self.conn.lock();
        let map = |r: &rusqlite::Row| -> rusqlite::Result<HistoryItem> {
            Ok(HistoryItem {
                id: r.get(0)?,
                created_at: r.get(1)?,
                text: r.get(2)?,
                app: r.get(3)?,
                style: r.get(4)?,
                delivery: r.get(5)?,
                duration_ms: r.get(6)?,
                words: r.get(7)?,
                pinned: r.get::<_, i64>(8)? != 0,
            })
        };
        const COLUMNS: &str = "h.id, h.created_at, h.text, h.app, h.style, h.delivery, h.duration_ms, h.words, h.pinned";
        let fts_query = fts_prefix_query(query);
        let rows = if fts_query.is_empty() {
            let mut stmt = conn.prepare(&format!(
                "SELECT {COLUMNS} FROM history h ORDER BY h.pinned DESC, h.created_at DESC LIMIT ?1"
            ))?;
            let rows = stmt.query_map([limit], map)?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare(&format!(
                "SELECT {COLUMNS} FROM history_fts f JOIN history h ON h.id = f.rowid
                 WHERE history_fts MATCH ?1 ORDER BY h.created_at DESC LIMIT ?2"
            ))?;
            let rows = stmt
                .query_map(params![fts_query, limit], map)?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        Ok(rows)
    }

    /// Apps dictated into recently, most used first (for style rules).
    pub fn recent_apps(&self, limit: i64) -> AppResult<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT app FROM history WHERE app <> '' AND created_at > ?1
             GROUP BY app ORDER BY COUNT(*) DESC, MAX(created_at) DESC LIMIT ?2",
        )?;
        let since = now_ms() - 90 * 86_400_000;
        let rows = stmt.query_map(params![since, limit], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn latest_text(&self) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .lock()
            .query_row("SELECT text FROM history ORDER BY created_at DESC LIMIT 1", [], |r| r.get(0))
            .optional()?)
    }

    pub fn history_text(&self, id: i64) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .lock()
            .query_row("SELECT text FROM history WHERE id = ?1", [id], |r| r.get(0))
            .optional()?)
    }

    pub fn update_history_text(&self, id: i64, text: &str) -> AppResult<()> {
        self.conn.lock().execute(
            "UPDATE history SET text = ?2, words = ?3 WHERE id = ?1",
            params![id, text, word_count(text)],
        )?;
        Ok(())
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> AppResult<()> {
        self.conn
            .lock()
            .execute("UPDATE history SET pinned = ?2 WHERE id = ?1", params![id, pinned as i64])?;
        Ok(())
    }

    pub fn delete_history(&self, id: i64) -> AppResult<()> {
        self.conn.lock().execute("DELETE FROM history WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn clear_history(&self, keep_pinned: bool) -> AppResult<()> {
        let sql = if keep_pinned {
            "DELETE FROM history WHERE pinned = 0"
        } else {
            "DELETE FROM history"
        };
        self.conn.lock().execute(sql, [])?;
        Ok(())
    }

    pub fn stats(&self) -> AppResult<Stats> {
        let conn = self.conn.lock();
        let (total_words, total_dictations, total_ms): (i64, i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(words),0), COUNT(*), COALESCE(SUM(duration_ms),0) FROM history WHERE delivery != 'imported'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let words_today: i64 = conn.query_row(
            "SELECT COALESCE(SUM(words),0) FROM history
             WHERE date(created_at/1000,'unixepoch','localtime') = date('now','localtime')",
            [],
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT date(created_at/1000,'unixepoch','localtime') AS day, SUM(words)
             FROM history GROUP BY day ORDER BY day DESC LIMIT 400",
        )?;
        let days: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let today: String = conn.query_row("SELECT date('now','localtime')", [], |r| r.get(0))?;
        drop(stmt);
        drop(conn);

        let (current_streak, longest_streak) = streaks(&days, &today);
        let average_wpm = if total_ms > 0 {
            (total_words as f64 / (total_ms as f64 / 60_000.0)).round() as i64
        } else {
            0
        };
        let typing_minutes = total_words as f64 / 40.0;
        let speaking_minutes = total_ms as f64 / 60_000.0;
        Ok(Stats {
            total_words,
            total_dictations,
            words_today,
            average_wpm,
            minutes_saved: (typing_minutes - speaking_minutes).max(0.0).round() as i64,
            current_streak,
            longest_streak,
            activity: days.into_iter().take(120).collect(),
        })
    }

    // ---- dictionary ------------------------------------------------------

    pub fn dictionary(&self) -> AppResult<Vec<DictionaryEntry>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT id, term, replacement, source FROM dictionary ORDER BY term COLLATE NOCASE")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(DictionaryEntry {
                    id: r.get(0)?,
                    term: r.get(1)?,
                    replacement: r.get(2)?,
                    source: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Insert or update by term (case-insensitive).
    pub fn upsert_dictionary(&self, term: &str, replacement: Option<&str>, source: &str) -> AppResult<()> {
        let term = term.trim();
        if term.is_empty() {
            return Ok(());
        }
        let replacement = replacement.map(str::trim).filter(|r| !r.is_empty());
        self.conn.lock().execute(
            "INSERT INTO dictionary(term, replacement, source, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(term) DO UPDATE SET replacement = excluded.replacement",
            params![term, replacement, source, now_ms()],
        )?;
        Ok(())
    }

    pub fn delete_dictionary(&self, id: i64) -> AppResult<()> {
        self.conn.lock().execute("DELETE FROM dictionary WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---- snippets --------------------------------------------------------

    pub fn snippets(&self) -> AppResult<Vec<Snippet>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, trigger, expansion FROM snippets ORDER BY trigger COLLATE NOCASE")?;
        let rows = stmt
            .query_map([], |r| Ok(Snippet { id: r.get(0)?, trigger: r.get(1)?, expansion: r.get(2)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn upsert_snippet(&self, trigger: &str, expansion: &str) -> AppResult<()> {
        let trigger = trigger.trim();
        if trigger.is_empty() || expansion.is_empty() {
            return Ok(());
        }
        self.conn.lock().execute(
            "INSERT INTO snippets(trigger, expansion, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(trigger) DO UPDATE SET expansion = excluded.expansion",
            params![trigger, expansion, now_ms()],
        )?;
        Ok(())
    }

    pub fn delete_snippet(&self, id: i64) -> AppResult<()> {
        self.conn.lock().execute("DELETE FROM snippets WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---- learned suggestions ---------------------------------------------

    pub fn add_suggestion(&self, heard: &str, correct: &str) -> AppResult<()> {
        self.conn.lock().execute(
            "INSERT INTO suggestions(heard, correct, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(heard, correct) DO UPDATE SET count = count + 1",
            params![heard, correct, now_ms()],
        )?;
        Ok(())
    }

    pub fn suggestions(&self) -> AppResult<Vec<Suggestion>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT id, heard, correct, count FROM suggestions ORDER BY count DESC, created_at DESC")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Suggestion { id: r.get(0)?, heard: r.get(1)?, correct: r.get(2)?, count: r.get(3)? })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Approving turns the suggestion into a dictionary replacement.
    pub fn resolve_suggestion(&self, id: i64, approve: bool) -> AppResult<()> {
        let found: Option<(String, String)> = self
            .conn
            .lock()
            .query_row("SELECT heard, correct FROM suggestions WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        if let (true, Some((heard, correct))) = (approve, found) {
            self.upsert_dictionary(&heard, Some(&correct), "learned")?;
        }
        self.conn.lock().execute("DELETE FROM suggestions WHERE id = ?1", [id])?;
        Ok(())
    }
}

/// Turn free text into a safe FTS5 query: each word quoted, with a prefix wildcard.
fn fts_prefix_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|word| word.replace('"', ""))
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{word}\"*"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `days` is sorted newest first ("YYYY-MM-DD"). A streak still counts if the
/// latest active day is yesterday (today isn't over yet).
fn streaks(days: &[(String, i64)], today: &str) -> (i64, i64) {
    fn ordinal(day: &str) -> Option<i64> {
        let mut parts = day.split('-').map(|p| p.parse::<i64>().ok());
        let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
        // Days since civil epoch (Howard Hinnant's algorithm).
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(era * 146_097 + doe)
    }
    let ordinals: Vec<i64> = days.iter().filter_map(|(day, _)| ordinal(day)).collect();
    let Some(today) = ordinal(today) else { return (0, 0) };

    let mut longest = 0;
    let mut run = 0;
    let mut previous: Option<i64> = None;
    for &day in &ordinals {
        run = match previous {
            Some(p) if p - day == 1 => run + 1,
            _ => 1,
        };
        longest = longest.max(run);
        previous = Some(day);
    }

    let mut current = 0;
    if let Some(&first) = ordinals.first() {
        if today - first <= 1 {
            current = 1;
            for pair in ordinals.windows(2) {
                if pair[0] - pair[1] == 1 {
                    current += 1;
                } else {
                    break;
                }
            }
        }
    }
    (current, longest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_roundtrip_and_search() {
        let store = Store::open_in_memory().unwrap();
        store
            .add_history(&NewHistory { text: "hello world from hams", delivery: "pasted", duration_ms: 3000, ..Default::default() })
            .unwrap();
        let id = store
            .add_history(&NewHistory { text: "second note about groceries", ..Default::default() })
            .unwrap();
        assert_eq!(store.history("", 10).unwrap().len(), 2);
        let found = store.history("groc", 10).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);

        store.update_history_text(id, "edited text").unwrap();
        assert!(store.history("groc", 10).unwrap().is_empty());
        assert_eq!(store.history("edited", 10).unwrap().len(), 1);

        store.set_pinned(id, true).unwrap();
        assert!(store.history("", 10).unwrap()[0].pinned);
        store.delete_history(id).unwrap();
        assert_eq!(store.history("", 10).unwrap().len(), 1);
    }

    #[test]
    fn search_tolerates_fts_syntax() {
        let store = Store::open_in_memory().unwrap();
        store.add_history(&NewHistory { text: "quote \"me\" AND or", ..Default::default() }).unwrap();
        assert!(store.history("\"AND (", 10).is_ok());
    }

    #[test]
    fn dictionary_upsert_is_case_insensitive() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_dictionary("Tauri", None, "user").unwrap();
        store.upsert_dictionary("tauri", Some("Tauri 2"), "user").unwrap();
        let entries = store.dictionary().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].replacement.as_deref(), Some("Tauri 2"));
    }

    #[test]
    fn approving_a_suggestion_creates_a_replacement() {
        let store = Store::open_in_memory().unwrap();
        store.add_suggestion("hams", "Hams").unwrap();
        store.add_suggestion("hams", "Hams").unwrap();
        let suggestion = &store.suggestions().unwrap()[0];
        assert_eq!(suggestion.count, 2);
        store.resolve_suggestion(suggestion.id, true).unwrap();
        assert!(store.suggestions().unwrap().is_empty());
        assert_eq!(store.dictionary().unwrap()[0].replacement.as_deref(), Some("Hams"));
    }

    #[test]
    fn streak_counting() {
        let days = |list: &[&str]| list.iter().map(|d| (d.to_string(), 1)).collect::<Vec<_>>();
        assert_eq!(streaks(&days(&["2026-11-05", "2026-11-04", "2026-11-03"]), "2026-11-05"), (3, 3));
        assert_eq!(streaks(&days(&["2026-11-04", "2026-11-03"]), "2026-11-05"), (2, 2));
        assert_eq!(streaks(&days(&["2026-11-02", "2026-11-01"]), "2026-11-05"), (0, 2));
        assert_eq!(streaks(&days(&["2026-03-01", "2026-02-28"]), "2026-03-01"), (2, 2));
        assert_eq!(streaks(&[], "2026-11-05"), (0, 0));
    }

    #[test]
    fn kv_roundtrip() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.get_json::<u32>("x").unwrap(), None);
        store.set_json("x", &7u32).unwrap();
        store.set_json("x", &8u32).unwrap();
        assert_eq!(store.get_json::<u32>("x").unwrap(), Some(8));
    }
}
