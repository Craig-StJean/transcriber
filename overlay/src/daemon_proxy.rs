use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use tokio::task::JoinHandle;
use zbus::Connection;

// ── zbus proxy ────────────────────────────────────────────────────────────────

/// Client-side proxy for the daemon's DBus interface.
/// Matches the server-side `#[interface]` in `daemon/src/dbus.rs`.
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

    /// Cancel an in-progress recording or transcription.
    async fn cancel(&self) -> zbus::Result<()>;

    /// Emitted whenever the daemon state changes.
    /// Values: "Idle" | "Recording" | "Transcribing" | "PostProcessing" | "Streaming" | "Done" | "Error"
    #[zbus(signal)]
    async fn state_changed(&self, state: String) -> zbus::Result<()>;

    /// Emitted right before `StateChanged("Error")` with a human-readable reason.
    #[zbus(signal)]
    async fn error_occurred(&self, message: String) -> zbus::Result<()>;

    /// Emitted right before `StateChanged("Idle")` when a session ends
    /// without text. Reason: "cancelled" | "no-speech".
    #[zbus(signal)]
    async fn session_discarded(&self, reason: String) -> zbus::Result<()>;

    /// Audio VU level in range 0.0–1.0, emitted ~30 Hz while recording.
    #[zbus(signal)]
    async fn audio_level(&self, level: f64) -> zbus::Result<()>;

    /// Emitted after a successful transcription with the resulting text.
    #[zbus(signal)]
    async fn transcription_ready(&self, text: String) -> zbus::Result<()>;

    /// Emitted during a streaming transcription. Each emission carries the
    /// running cumulative transcript so far — clients track a "last text"
    /// cursor and inject only the new suffix.
    #[zbus(signal)]
    async fn transcription_chunk(&self, text: String) -> zbus::Result<()>;

    /// The daemon's current state string (same values as StateChanged signal).
    #[zbus(property)]
    fn current_state(&self) -> zbus::Result<String>;
}

// ── Event type sent from the tokio thread back to the GTK main thread ─────────

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    StateChanged(String),
    ErrorOccurred(String),
    SessionDiscarded(String),
    AudioLevel(f64),
    TranscriptionReady(String),
    TranscriptionChunk(String),
}

// ── Connection helpers ────────────────────────────────────────────────────────

/// Build a proxy for the daemon on the session bus.
///
/// Property caching is disabled: the daemon may restart underneath us, and a
/// stale cached `CurrentState` is exactly what would desync the hotkey toggle.
/// Every read goes to the bus instead (it's only done on (re)connect).
pub async fn proxy(conn: &Connection) -> zbus::Result<DaemonProxy<'static>> {
    DaemonProxy::builder(conn)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
}

/// Aborts every forwarding task when dropped, so a re-subscription (or the
/// whole session going away) never leaves stale forwarders running.
pub struct Subscription(Vec<JoinHandle<()>>);

impl Drop for Subscription {
    fn drop(&mut self) {
        for h in &self.0 {
            h.abort();
        }
    }
}

/// Publish a state both to the shared toggle state and to the GTK thread.
pub async fn publish_state(
    state: String,
    event_tx: &async_channel::Sender<DaemonEvent>,
    current_state: &Mutex<String>,
) {
    *current_state.lock().unwrap() = state.clone();
    event_tx.send(DaemonEvent::StateChanged(state)).await.ok();
}

/// Re-read the daemon's `CurrentState` and publish it. Called after every
/// (re)subscription so the overlay and hotkey toggle match reality.
///
/// Only call this while the name has an owner: reading a property of an
/// unowned name would DBus-activate the daemon as a side effect.
pub async fn sync_state(
    proxy: &DaemonProxy<'static>,
    event_tx: &async_channel::Sender<DaemonEvent>,
    current_state: &Mutex<String>,
) {
    let state = match proxy.current_state().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("could not read CurrentState: {e}");
            "Idle".to_string()
        }
    };
    publish_state(state, event_tx, current_state).await;
}

/// Subscribe to every daemon signal and spawn tasks forwarding them to the
/// GTK main thread via `event_tx`. Dropping the returned `Subscription`
/// stops the forwarders.
pub async fn subscribe(
    proxy: &DaemonProxy<'static>,
    event_tx: &async_channel::Sender<DaemonEvent>,
    current_state: &Arc<Mutex<String>>,
) -> zbus::Result<Subscription> {
    let mut handles = Vec::with_capacity(6);

    // ── StateChanged ──────────────────────────────────────────────────────────
    let tx = event_tx.clone();
    let state = current_state.clone();
    let mut stream = proxy.receive_state_changed().await?;
    handles.push(tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                publish_state(args.state().to_owned(), &tx, &state).await;
            }
        }
    }));

    // ── ErrorOccurred ─────────────────────────────────────────────────────────
    let tx = event_tx.clone();
    let mut stream = proxy.receive_error_occurred().await?;
    handles.push(tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                tx.send(DaemonEvent::ErrorOccurred(args.message().to_owned())).await.ok();
            }
        }
    }));

    // ── SessionDiscarded ──────────────────────────────────────────────────────
    let tx = event_tx.clone();
    let mut stream = proxy.receive_session_discarded().await?;
    handles.push(tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                tx.send(DaemonEvent::SessionDiscarded(args.reason().to_owned())).await.ok();
            }
        }
    }));

    // ── AudioLevel ────────────────────────────────────────────────────────────
    let tx = event_tx.clone();
    let mut stream = proxy.receive_audio_level().await?;
    handles.push(tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                tx.send(DaemonEvent::AudioLevel(*args.level())).await.ok();
            }
        }
    }));

    // ── TranscriptionReady ────────────────────────────────────────────────────
    let tx = event_tx.clone();
    let mut stream = proxy.receive_transcription_ready().await?;
    handles.push(tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                tx.send(DaemonEvent::TranscriptionReady(args.text().to_owned())).await.ok();
            }
        }
    }));

    // ── TranscriptionChunk (streaming providers only) ─────────────────────────
    let tx = event_tx.clone();
    let mut stream = proxy.receive_transcription_chunk().await?;
    handles.push(tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                tx.send(DaemonEvent::TranscriptionChunk(args.text().to_owned())).await.ok();
            }
        }
    }));

    Ok(Subscription(handles))
}
