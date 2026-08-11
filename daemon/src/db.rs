use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id:        i64,
    pub timestamp: i64,
    pub text:      String,
    pub status:    String,          // "ok" | "failed"
    pub error:     Option<String>,
    pub wav_path:  Option<String>,
    /// Raw transcription before LLM post-processing was applied. None when
    /// post-processing was off (in which case `text` is already the raw output).
    pub text_original: Option<String>,
}

/// Wrapping `Connection` in `Mutex` makes `Database: Send + Sync`.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = common::config::data_dir().join("history.db");

        std::fs::create_dir_all(path.parent().unwrap())?;

        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                text      TEXT    NOT NULL DEFAULT '',
                timestamp INTEGER NOT NULL,
                status    TEXT    NOT NULL DEFAULT 'ok',
                error     TEXT,
                wav_path  TEXT
             );",
        )?;

        // Migrate existing databases that lack the newer columns.
        // SQLite returns an error if a column already exists — ignore it.
        for sql in &[
            "ALTER TABLE history ADD COLUMN status        TEXT NOT NULL DEFAULT 'ok'",
            "ALTER TABLE history ADD COLUMN error         TEXT",
            "ALTER TABLE history ADD COLUMN wav_path      TEXT",
            "ALTER TABLE history ADD COLUMN text_original TEXT",
        ] {
            let _ = conn.execute(sql, []);
        }

        tracing::info!("history database opened at {}", path.display());
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Insert a successful transcription entry. `text_original` is the raw
    /// transcript before post-processing; `None` when post-processing was off.
    pub fn insert(&self, text: &str, text_original: Option<&str>, wav_path: Option<&str>) -> Result<()> {
        let now = now_secs()?;
        self.conn.lock().unwrap().execute(
            "INSERT INTO history (text, timestamp, status, wav_path, text_original)
             VALUES (?1, ?2, 'ok', ?3, ?4)",
            params![text, now, wav_path, text_original],
        )?;
        Ok(())
    }

    /// Insert a failed transcription entry (no text).
    pub fn insert_failed(&self, error: &str, wav_path: Option<&str>) -> Result<()> {
        let now = now_secs()?;
        self.conn.lock().unwrap().execute(
            "INSERT INTO history (text, timestamp, status, error, wav_path)
             VALUES ('', ?1, 'failed', ?2, ?3)",
            params![now, error, wav_path],
        )?;
        Ok(())
    }

    /// Update an existing entry after a successful retry.
    pub fn update_retry_success(&self, id: i64, text: &str) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "UPDATE history SET text = ?1, status = 'ok', error = NULL WHERE id = ?2",
            params![text, id],
        )?;
        Ok(())
    }

    /// Update an existing entry after a failed retry.
    pub fn update_retry_failed(&self, id: i64, error: &str) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "UPDATE history SET status = 'failed', error = ?1 WHERE id = ?2",
            params![error, id],
        )?;
        Ok(())
    }

    /// Delete a single entry by id.
    pub fn delete_entry(&self, id: i64) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "DELETE FROM history WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Delete all history entries.
    pub fn clear_all(&self) -> Result<()> {
        self.conn.lock().unwrap().execute("DELETE FROM history", [])?;
        Ok(())
    }

    /// Return the most recent `limit` entries, newest first.
    pub fn recent(&self, limit: u32) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, text, status, error, wav_path, text_original
             FROM history ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(HistoryEntry {
                id:            row.get(0)?,
                timestamp:     row.get(1)?,
                text:          row.get(2)?,
                status:        row.get(3)?,
                error:         row.get(4)?,
                wav_path:      row.get(5)?,
                text_original: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn now_secs() -> Result<i64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64)
}
