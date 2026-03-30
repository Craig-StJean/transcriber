use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use zbus::Connection;

// ── zbus proxy ────────────────────────────────────────────────────────────────

/// Client-side proxy for the daemon's DBus interface.
/// Matches the server-side `#[interface]` in `daemon/src/dbus.rs` exactly.
#[zbus::proxy(
    interface = "org.transcriber.Daemon",
    default_service = "org.transcriber.Daemon",
    default_path = "/org/transcriber/Daemon",
    gen_blocking = false,
)]
pub trait Daemon {
    /// Begin capturing audio from the default microphone.
    async fn start_recording(&self) -> zbus::Result<()>;

    /// Stop recording and send audio to the transcription API.
    async fn stop_recording(&self) -> zbus::Result<()>;

    /// Cancel an in-progress recording without transcribing.
    async fn cancel(&self) -> zbus::Result<()>;

    /// Emitted whenever the daemon state changes.
    /// Values: "Idle" | "Recording" | "Transcribing" | "Done" | "Error"
    #[zbus(signal)]
    async fn state_changed(&self, state: String) -> zbus::Result<()>;

    /// Audio VU level in range 0.0–1.0, emitted ~30 Hz while recording.
    #[zbus(signal)]
    async fn audio_level(&self, level: f64) -> zbus::Result<()>;

    /// Emitted after a successful transcription with the resulting text.
    #[zbus(signal)]
    async fn transcription_ready(&self, text: String) -> zbus::Result<()>;

    /// The daemon's current state string (same values as StateChanged signal).
    /// Declared as a regular fn: zbus caches property values and exposes them
    /// synchronously in proxies generated with gen_blocking = false.
    #[zbus(property)]
    fn current_state(&self) -> zbus::Result<String>;
}

// ── Event type sent from the tokio thread back to the GTK main thread ─────────

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    StateChanged(String),
    AudioLevel(f64),
    TranscriptionReady(String),
}

// ── Connection helper ─────────────────────────────────────────────────────────

/// Connect to the daemon's DBus service, seed the initial state, and spawn
/// background tasks that forward signals to the GTK main thread via `event_tx`.
///
/// Returns the proxy so the caller can issue method calls (e.g. from the
/// GlobalShortcuts portal activation handler).
pub async fn connect(
    event_tx: async_channel::Sender<DaemonEvent>,
    current_state: Arc<Mutex<String>>,
) -> anyhow::Result<DaemonProxy<'static>> {
    let conn = Connection::session().await?;
    let proxy = DaemonProxy::new(&conn).await?;

    // Seed the overlay with the daemon's current state so toggle works
    // immediately even before the first StateChanged signal fires.
    let init = proxy.current_state().await.unwrap_or_else(|_| "Idle".to_string());
    *current_state.lock().unwrap() = init.clone();
    event_tx.send(DaemonEvent::StateChanged(init)).await.ok();

    // ── StateChanged stream ───────────────────────────────────────────────────
    let tx1 = event_tx.clone();
    let state1 = current_state.clone();
    let mut state_stream = proxy.receive_state_changed().await?;
    tokio::spawn(async move {
        while let Some(signal) = state_stream.next().await {
            if let Ok(args) = signal.args() {
                let s = args.state().to_owned();
                *state1.lock().unwrap() = s.clone();
                tx1.send(DaemonEvent::StateChanged(s)).await.ok();
            }
        }
        tracing::info!("StateChanged stream ended — daemon likely stopped");
    });

    // ── AudioLevel stream ─────────────────────────────────────────────────────
    let tx2 = event_tx.clone();
    let mut level_stream = proxy.receive_audio_level().await?;
    tokio::spawn(async move {
        while let Some(signal) = level_stream.next().await {
            if let Ok(args) = signal.args() {
                tx2.send(DaemonEvent::AudioLevel(*args.level())).await.ok();
            }
        }
    });

    // ── TranscriptionReady stream ─────────────────────────────────────────────
    let tx3 = event_tx.clone();
    let mut text_stream = proxy.receive_transcription_ready().await?;
    tokio::spawn(async move {
        while let Some(signal) = text_stream.next().await {
            if let Ok(args) = signal.args() {
                tx3.send(DaemonEvent::TranscriptionReady(args.text().to_owned())).await.ok();
            }
        }
    });

    Ok(proxy)
}
