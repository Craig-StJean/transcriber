mod daemon_proxy;
mod overlay_window;
mod shortcuts;

use std::cell::Cell;
use std::sync::{Arc, Mutex};

use daemon_proxy::DaemonEvent;
use futures_util::StreamExt;
use gtk4::glib;
use gtk4::prelude::*;

/// Longest error message shown in the overlay; the full text is in the
/// daemon's journal.
const MAX_ERROR_CHARS: usize = 80;

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app = gtk4::Application::builder()
        .application_id("org.transcriber.Overlay")
        .build();

    // GApplication re-emits `activate` on the primary instance if the binary
    // is launched a second time; only set up once.
    let activated = Cell::new(false);

    app.connect_activate(move |app| {
        if activated.replace(true) {
            return;
        }

        // Without wlr-layer-shell there is nothing this process can do. Exit
        // cleanly (status 0) so a `Restart=on-failure` unit doesn't loop — this
        // is the expected outcome when the unit is enabled on GNOME.
        if !gtk4_layer_shell::is_supported() {
            tracing::error!(
                "compositor does not support wlr-layer-shell — \
                 this overlay only works on wlroots-based compositors \
                 (Sway, Hyprland, KDE Plasma 6, …). \
                 On GNOME, use the Shell Extension instead. Exiting."
            );
            app.quit();
            return;
        }

        // async-channel bridges the tokio thread ↔ glib executor.
        let (event_tx, event_rx) = async_channel::unbounded::<DaemonEvent>();

        // Shared current state — written by the signal-forwarding tokio tasks,
        // read by the shortcut activation handler (both in tokio context).
        let current_state: Arc<Mutex<String>> = Arc::new(Mutex::new("Idle".to_string()));

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(run_async(event_tx, current_state));
        });

        let overlay = overlay_window::OverlayWindow::new(app);
        glib::MainContext::default().spawn_local(handle_events(event_rx, overlay));
    });

    app.run()
}

/// GTK-thread event loop: drives the overlay and performs text output.
async fn handle_events(
    event_rx: async_channel::Receiver<DaemonEvent>,
    overlay: overlay_window::OverlayWindow,
) {
    // Re-read at the start of every recording so toggling it in the settings
    // app takes effect without restarting the overlay.
    let mut direct_injection = load_direct_injection();
    // Cursor into the running streaming transcript — chunks are cumulative,
    // so we type only the new suffix on each emission.
    let mut last_chunk = String::new();
    // Set when typing a chunk failed; the final text then goes to the
    // clipboard instead of being typed (partially) again.
    let mut inject_failed = false;
    // Whether the last result was typed (vs copied) — picks the Done label.
    let mut typed = false;
    // Latest ErrorOccurred message, shown on the following Error state.
    let mut last_error: Option<String> = None;
    // Last state seen, used to tell a cancel (active → Idle) from a plain Idle.
    let mut prev_state = String::from("Idle");

    while let Ok(event) = event_rx.recv().await {
        match event {
            DaemonEvent::StateChanged(state) => {
                match state.as_str() {
                    "Recording" => {
                        direct_injection = load_direct_injection();
                        last_chunk.clear();
                        inject_failed = false;
                        typed = false;
                        last_error = None;
                        overlay.handle_state(&state);
                    }
                    "Done" => {
                        overlay.show_message(if typed { "✓ Typed!" } else { "✓ Copied!" }, 800);
                    }
                    "Error" => {
                        let msg = match last_error.take() {
                            Some(m) => format!("✗ {}", truncate(&m, MAX_ERROR_CHARS)),
                            None => "✗ Error — check logs".to_string(),
                        };
                        overlay.show_message(&msg, 3500);
                    }
                    "Idle" => {
                        last_chunk.clear();
                        if is_active(&prev_state) {
                            // Active → Idle without Done/Error: the recording
                            // was cancelled (or was empty and discarded).
                            overlay.show_message("Cancelled", 800);
                        } else if prev_state == "Idle" {
                            overlay.handle_state(&state);
                        }
                        // From Done/Error: that message is already fading.
                    }
                    _ => overlay.handle_state(&state),
                }
                prev_state = state;
            }
            DaemonEvent::ErrorOccurred(message) => {
                tracing::warn!("daemon error: {message}");
                last_error = Some(message);
            }
            DaemonEvent::AudioLevel(level) => {
                overlay.set_level(level);
            }
            DaemonEvent::TranscriptionChunk(text) => {
                // Only live states accept chunks: anything arriving after a
                // cancel (Idle) or a finished run is stale and must not be typed.
                // In clipboard mode the final TranscriptionReady wins; skip.
                if !matches!(prev_state.as_str(), "Recording" | "Streaming")
                    || !direct_injection
                    || inject_failed
                {
                    continue;
                }
                if let Some(delta) = text.strip_prefix(last_chunk.as_str()) {
                    if !delta.is_empty() {
                        match type_text(delta) {
                            Ok(()) => typed = true,
                            Err(e) => {
                                tracing::warn!(
                                    "direct injection (chunk) failed: {e} — \
                                     final text will go to the clipboard"
                                );
                                inject_failed = true;
                            }
                        }
                    }
                    last_chunk = text;
                }
            }
            DaemonEvent::TranscriptionReady(text) => {
                // A result after a cancel is stale.
                if prev_state == "Idle" {
                    continue;
                }
                if direct_injection && !inject_failed {
                    // If streaming already typed everything, this is a no-op.
                    // Otherwise (batch path) type the whole thing.
                    let to_type = text.strip_prefix(last_chunk.as_str()).unwrap_or(&text);
                    if to_type.is_empty() {
                        typed = true;
                    } else {
                        match type_text(to_type) {
                            Ok(()) => typed = true,
                            Err(e) => {
                                tracing::warn!(
                                    "direct injection failed: {e} — copying to clipboard instead"
                                );
                                copy_to_clipboard(&text);
                                typed = false;
                            }
                        }
                    }
                } else {
                    copy_to_clipboard(&text);
                    typed = false;
                }
                last_chunk.clear();
            }
        }
    }
}

fn is_active(state: &str) -> bool {
    matches!(state, "Recording" | "Transcribing" | "Streaming" | "PostProcessing")
}

fn load_direct_injection() -> bool {
    match common::config::load() {
        Ok(c) => c.direct_injection,
        Err(e) => {
            tracing::warn!("could not read config ({e}); assuming clipboard mode");
            false
        }
    }
}

/// Type `text` into the focused window through a Wayland virtual keyboard.
fn type_text(text: &str) -> anyhow::Result<()> {
    use enigo::{Enigo, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.text(text)?;
    Ok(())
}

/// Put `text` on the Wayland clipboard via GDK. This process holds its own
/// Wayland connection, so no helper process is needed.
fn copy_to_clipboard(text: &str) {
    match gtk4::gdk::Display::default() {
        Some(display) => display.clipboard().set_text(text),
        None => tracing::error!("no GDK display — cannot copy transcription to clipboard"),
    }
}

/// Truncate to at most `max` characters (not bytes), adding an ellipsis.
fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    match s.char_indices().nth(max) {
        Some((idx, _)) => format!("{}…", s[..idx].trim_end()),
        None => s.to_string(),
    }
}

/// Runs in a dedicated thread under the Tokio runtime. Reconnects to the
/// session bus if the connection is ever lost.
async fn run_async(
    event_tx: async_channel::Sender<DaemonEvent>,
    current_state: Arc<Mutex<String>>,
) {
    loop {
        if let Err(e) = run_session(&event_tx, &current_state).await {
            tracing::error!("session bus connection failed: {e}");
        }
        daemon_proxy::publish_state("Idle".into(), &event_tx, &current_state).await;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

/// One session-bus connection's lifetime. Follows the daemon across restarts
/// by watching `NameOwnerChanged` for `org.transcriber.Daemon`: each new
/// owner gets a fresh signal subscription and a re-read of `CurrentState`;
/// when the name vanishes the overlay drops to Idle.
///
/// Only returns when the bus connection itself is gone.
async fn run_session(
    event_tx: &async_channel::Sender<DaemonEvent>,
    current_state: &Arc<Mutex<String>>,
) -> anyhow::Result<()> {
    let conn = zbus::Connection::session().await?;
    let proxy = daemon_proxy::proxy(&conn).await?;
    tracing::info!("connected to session bus");

    // Hotkeys live in their own task so a missing portal never blocks
    // following the daemon. Aborted when this function returns.
    let shortcuts_task = tokio::spawn({
        let proxy = proxy.clone();
        let state = current_state.clone();
        async move {
            match shortcuts::run(proxy, state).await {
                Ok(()) => tracing::warn!("global shortcuts stream ended"),
                Err(e) => tracing::warn!("global shortcuts unavailable: {e}"),
            }
        }
    });
    let _abort_shortcuts = AbortOnDrop(shortcuts_task);

    // Subscribe to owner changes *before* checking the owner, so a daemon
    // starting in between can't be missed.
    let mut owner_changes = proxy.inner().receive_owner_changed().await?;

    let dbus = zbus::fdo::DBusProxy::new(&conn).await?;
    let has_owner = dbus
        .name_has_owner(common::dbus::SERVICE_NAME.try_into()?)
        .await
        .unwrap_or(false);

    // Subscribe first, then read the state, so no transition is lost between.
    let mut _subscription = Some(daemon_proxy::subscribe(&proxy, event_tx, current_state).await?);
    if has_owner {
        tracing::info!("daemon is running");
        daemon_proxy::sync_state(&proxy, event_tx, current_state).await;
    } else {
        tracing::info!("daemon not running yet — waiting for it to appear");
        daemon_proxy::publish_state("Idle".into(), event_tx, current_state).await;
    }

    while let Some(owner) = owner_changes.next().await {
        // Drop (abort) the old forwarders before anything else.
        _subscription = None;
        match owner {
            Some(name) => {
                tracing::info!("daemon appeared as {name}; re-subscribing");
                _subscription = Some(daemon_proxy::subscribe(&proxy, event_tx, current_state).await?);
                daemon_proxy::sync_state(&proxy, event_tx, current_state).await;
            }
            None => {
                tracing::info!("daemon left the bus");
                daemon_proxy::publish_state("Idle".into(), event_tx, current_state).await;
            }
        }
    }

    anyhow::bail!("NameOwnerChanged stream ended")
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_keeps_short_messages() {
        assert_eq!(truncate("  API key missing ", 80), "API key missing");
    }

    #[test]
    fn truncate_counts_chars_not_bytes() {
        assert_eq!(truncate("ééééé", 3), "ééé…");
    }
}
