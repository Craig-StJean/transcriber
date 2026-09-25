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

    /// Case-insensitive match against everything the row can display.
    /// `needle` must already be lowercase.
    pub fn matches(&self, needle: &str) -> bool {
        needle.is_empty()
            || self.text.to_lowercase().contains(needle)
            || self.text_original.as_deref().is_some_and(|t| t.to_lowercase().contains(needle))
            || self.error.as_deref().is_some_and(|t| t.to_lowercase().contains(needle))
    }
}

#[derive(Default)]
pub struct HistoryDb {
    conn:         Option<Connection>,
    data_version: Option<i64>,
}

impl HistoryDb {
    fn conn(&mut self) -> Option<&Connection> {
        if self.conn.is_none() {
            let path = config::data_dir().join("history.db");
            // Don't create it: an empty file here would be a database with
            // no schema, which the daemon would then have to cope with.
            if !path.exists() {
                return None;
            }
            self.conn = Connection::open_with_flags(
                &path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .ok();
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
        let Ok(mut stmt) = conn.prepare_cached(
            "SELECT id, timestamp, text, status, error, wav_path, text_original
             FROM history ORDER BY id DESC LIMIT ?1",
        ) else {
            return vec![];
        };
        stmt.query_map([LOAD_LIMIT], |row| {
            Ok(HistoryEntry {
                id:            row.get(0)?,
                timestamp:     row.get(1)?,
                text:          row.get(2)?,
                status:        row.get(3)?,
                error:         row.get(4)?,
                wav_path:      row.get(5)?,
                text_original: row.get(6)?,
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }
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
