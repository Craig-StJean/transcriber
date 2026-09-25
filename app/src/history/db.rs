//! Read-only view of the daemon's history database.
//!
//! The daemon owns the schema (and its migrations, in `daemon/src/db.rs`) and
//! all writes; deletes go through its DBus methods so the WAVs are removed
//! with the rows. This side only ever reads.

use common::config;
use rusqlite::{Connection, OpenFlags};

/// More than this and building rows starts to cost noticeable time; it also
/// matches the daemon's default retention.
const LOAD_LIMIT: i64 = 500;

/// Cap on search results. A search is for finding one entry; past this the
/// query is too broad to scan by eye anyway.
const SEARCH_LIMIT: i64 = 200;

const COLUMNS: &str = "id, timestamp, text, status, error, wav_path, text_original";

#[derive(Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id:            i64,
    pub timestamp:     i64,
    pub text:          String,
    pub status:        String,
    pub error:         Option<String>,
    pub wav_path:      Option<String>,
    pub text_original: Option<String>,
}

impl HistoryEntry {
    pub fn failed(&self) -> bool {
        self.status == "failed"
    }

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(HistoryEntry {
            id:            row.get(0)?,
            timestamp:     row.get(1)?,
            text:          row.get(2)?,
            status:        row.get(3)?,
            error:         row.get(4)?,
            wav_path:      row.get(5)?,
            text_original: row.get(6)?,
        })
    }
}

fn db_path() -> Option<std::path::PathBuf> {
    let path = config::data_dir().join("history.db");
    // Don't create it: an empty file here would be a database with no
    // schema, which the daemon would then have to cope with.
    path.exists().then_some(path)
}

fn open_read_only(path: &std::path::Path) -> Option<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()
}

#[derive(Default)]
pub struct HistoryDb {
    conn:         Option<Connection>,
    data_version: Option<i64>,
}

impl HistoryDb {
    fn conn(&mut self) -> Option<&Connection> {
        if self.conn.is_none() {
            self.conn = open_read_only(&db_path()?);
        }
        self.conn.as_ref()
    }

    /// Whether another connection (the daemon) has committed since the last
    /// call. `PRAGMA data_version` changes on *any* commit by another
    /// connection — inserts, retries rewriting a row in place, deletes,
    /// retention pruning — which a `MAX(id)` poll would miss for all but the
    /// first. It needs a long-lived connection, hence the cached `conn`.
    pub fn changed(&mut self) -> bool {
        let Some(conn) = self.conn() else { return false };
        let Ok(v) = conn.query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0)) else {
            return false;
        };
        let changed = self.data_version != Some(v);
        self.data_version = Some(v);
        changed
    }

    /// Newest first.
    pub fn load(&mut self) -> Vec<HistoryEntry> {
        let Some(conn) = self.conn() else { return vec![] };
        let Ok(mut stmt) = conn.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM history ORDER BY id DESC LIMIT ?1"
        )) else {
            return vec![];
        };
        stmt.query_map([LOAD_LIMIT], HistoryEntry::from_row)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }
}

/// Entries whose text, original text or error contains `query`, newest
/// first. Searches the whole table, not just the rows `load` returns.
/// Opens its own connection so it can run on a worker thread; call it off
/// the main thread. Case-insensitive for ASCII only (SQLite's `LIKE`).
pub fn search(query: &str) -> Vec<HistoryEntry> {
    let Some(conn) = db_path().as_deref().and_then(open_read_only) else { return vec![] };
    let pattern = format!("%{}%", escape_like(query));
    let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM history
         WHERE text LIKE ?1 ESCAPE '\\'
            OR text_original LIKE ?1 ESCAPE '\\'
            OR error LIKE ?1 ESCAPE '\\'
         ORDER BY id DESC LIMIT ?2"
    )) else {
        return vec![];
    };
    stmt.query_map(rusqlite::params![pattern, SEARCH_LIMIT], HistoryEntry::from_row)
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Make `%`, `_` and the escape character itself match literally.
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Length of a PCM WAV from its header. Does file I/O — call off the main
/// thread.
pub fn wav_duration_secs(path: &str) -> Option<f64> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 44];
    f.read_exact(&mut header).ok()?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return None;
    }
    let channels        = u16::from_le_bytes([header[22], header[23]]) as u32;
    let sample_rate     = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    let bits_per_sample = u16::from_le_bytes([header[34], header[35]]) as u32;
    let data_size       = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
    if sample_rate == 0 || channels == 0 || bits_per_sample == 0 {
        return None;
    }
    let bytes_per_second = sample_rate * channels * (bits_per_sample / 8);
    Some(data_size as f64 / bytes_per_second as f64)
}

#[cfg(test)]
mod tests {
    use super::escape_like;

    #[test]
    fn like_wildcards_match_literally() {
        assert_eq!(escape_like("100%_done\\"), "100\\%\\_done\\\\");
        assert_eq!(escape_like("plain"), "plain");
    }
}
