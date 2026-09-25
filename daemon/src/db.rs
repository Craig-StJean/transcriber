use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
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

    /// Update an existing entry after a successful retry or re-polish.
    /// `text_original` has the same meaning as in `insert` and is overwritten
    /// either way, so a retry with post-processing off doesn't leave a stale
    /// polished/raw pair.
    pub fn update_retry_success(&self, id: i64, text: &str, text_original: Option<&str>) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "UPDATE history SET text = ?1, text_original = ?2, status = 'ok', error = NULL
             WHERE id = ?3",
            params![text, text_original, id],
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

    /// The stored WAV path for an entry, if the entry exists and has one.
    pub fn wav_path(&self, id: i64) -> Result<Option<String>> {
        Ok(self.conn.lock().unwrap()
            .query_row("SELECT wav_path FROM history WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?
            .flatten())
    }

    /// An entry's `(text, text_original)`, or None if there is no such entry.
    pub fn texts(&self, id: i64) -> Result<Option<(String, Option<String>)>> {
        Ok(self.conn.lock().unwrap()
            .query_row(
                "SELECT text, text_original FROM history WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    /// Delete a single entry by id, and its WAV.
    pub fn delete_entry(&self, id: i64) -> Result<()> {
        let wavs = self.delete_where("id = ?1", params![id])?;
        remove_recordings(&wavs);
        Ok(())
    }

    /// Delete all history entries and their WAVs.
    pub fn clear_all(&self) -> Result<()> {
        let wavs = self.delete_where("1", [])?;
        remove_recordings(&wavs);
        Ok(())
    }

    /// Keep only the newest `keep` entries (0 = keep everything), deleting the
    /// rest along with their WAVs. Run after every insert so the recordings
    /// directory can't grow without bound.
    pub fn prune(&self, keep: u32) -> Result<()> {
        if keep == 0 {
            return Ok(());
        }
        let wavs = self.delete_where(
            "id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?1)",
            params![keep],
        )?;
        if !wavs.is_empty() {
            tracing::info!("history retention: removed {} old recording(s)", wavs.len());
        }
        remove_recordings(&wavs);
        Ok(())
    }

    /// Delete the rows matching `filter`, returning the WAV paths they held.
    /// Both statements run in one transaction so a concurrent insert can't
    /// slip in between and lose its WAV to the DELETE without being listed.
    fn delete_where(&self, filter: &str, args: impl rusqlite::Params + Clone) -> Result<Vec<String>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let wavs = {
            let mut stmt = tx.prepare(&format!(
                "SELECT wav_path FROM history WHERE wav_path IS NOT NULL AND ({filter})"
            ))?;
            let rows = stmt.query_map(args.clone(), |r| r.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        tx.execute(&format!("DELETE FROM history WHERE {filter}"), args)?;
        tx.commit()?;
        Ok(wavs)
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

/// Resolve `path` and return it only if it lies inside our own
/// `recordings/` directory.
///
/// WAV paths reach us from the DB (and historically from DBus callers), so
/// anything we read or delete is confined here: a bad row must not be able to
/// make the daemon read or unlink an arbitrary file.
pub fn recording_path(path: &Path) -> Option<PathBuf> {
    let dir = common::config::data_dir().join("recordings").canonicalize().ok()?;
    let resolved = path.canonicalize().ok()?;
    resolved.starts_with(&dir).then_some(resolved)
}

fn remove_recordings(paths: &[String]) {
    for p in paths {
        let Some(resolved) = recording_path(Path::new(p)) else {
            // Already gone, or outside recordings/ (e.g. pre-rename data dir).
            continue;
        };
        if let Err(e) = std::fs::remove_file(&resolved) {
            tracing::warn!("failed to delete recording {}: {e}", resolved.display());
        }
    }
}

fn now_secs() -> Result<i64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64)
}
