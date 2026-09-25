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
    // The streaming transcript typed so far this session — chunks are
    // cumulative, so each one types only its suffix beyond this.
    let mut stream_typed = String::new();
    // Whether any chunk was actually typed this session. The final text is
    // then only ever completed, never retyped from scratch.
    let mut typed_via_chunks = false;
    // Set when typing a chunk failed; the final text then goes to the
    // clipboard instead of being typed (partially) again.
    let mut inject_failed = false;
    // How the last result was delivered — picks the Done label.
    let mut output = Output::Copied;
    // Latest ErrorOccurred message, shown on the following Error state.
    let mut last_error: Option<String> = None;
    // Set by SessionDiscarded, which precedes the Idle it explains: that
    // Idle must not cut the message short.
    let mut discard_shown = false;
    // Last state seen, used to drop stale chunks/results.
    let mut prev_state = String::from("Idle");

    while let Ok(event) = event_rx.recv().await {
        match event {
            DaemonEvent::StateChanged(state) => {
                match state.as_str() {
                    "Recording" => {
                        direct_injection = load_direct_injection();
                        stream_typed.clear();
                        typed_via_chunks = false;
                        inject_failed = false;
                        output = Output::Copied;
                        last_error = None;
                        discard_shown = false;
                        overlay.handle_state(&state);
                    }
                    "Done" => {
                        let msg = match output {
                            Output::Typed => "✓ Typed!",
                            Output::Copied => "✓ Copied!",
                            Output::Mismatch => "Copied (stream mismatch)",
                        };
                        overlay.show_message(msg, 800);
                    }
                    "Error" => {
                        let msg = match last_error.take() {
                            Some(m) => format!("✗ {}", truncate(&m, MAX_ERROR_CHARS)),
                            None => "✗ Error — check logs".to_string(),
                        };
                        overlay.show_message(&msg, 3500);
                    }
                    "Idle" => {
                        if discard_shown {
                            // "Cancelled"/"No speech detected" is showing.
                            discard_shown = false;
                        } else if !matches!(prev_state.as_str(), "Done" | "Error") {
                            // Any other Idle (e.g. the daemon restarted
                            // mid-recording) just hides the overlay. After
                            // Done/Error that message is already fading.
                            overlay.handle_state(&state);
                        }
                    }
                    _ => overlay.handle_state(&state),
                }
                prev_state = state;
            }
            DaemonEvent::ErrorOccurred(message) => {
                tracing::warn!("daemon error: {message}");
                last_error = Some(message);
            }
            DaemonEvent::SessionDiscarded(reason) => match reason.as_str() {
                "cancelled" => {
                    overlay.show_message("Cancelled", 800);
                    discard_shown = true;
                }
                "no-speech" => {
                    overlay.show_message("No speech detected", 1200);
                    discard_shown = true;
                }
                other => tracing::warn!("unknown SessionDiscarded reason {other:?}"),
            },
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
                let Some(delta) = text.strip_prefix(stream_typed.as_str()) else {
                    tracing::warn!(
                        "stream chunk ({} chars) does not extend the {} chars already typed \
                         — not typing it",
                        text.len(),
                        stream_typed.len()
                    );
                    continue;
                };
                if delta.is_empty() {
                    continue;
                }
                match type_text(delta) {
                    Ok(()) => {
                        typed_via_chunks = true;
                        stream_typed = text;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "direct injection (chunk) failed: {e} — \
                             final text will go to the clipboard"
                        );
                        inject_failed = true;
                    }
                }
            }
            DaemonEvent::TranscriptionReady(text) => {
                // A result after a cancel is stale.
                if prev_state == "Idle" {
                    continue;
                }
                output = if !direct_injection || inject_failed {
                    copy_to_clipboard(&text);
                    Output::Copied
                } else if typed_via_chunks {
                    // Streaming already typed a prefix: complete it, but
                    // never retype text that's already in the window.
                    match text.strip_prefix(stream_typed.as_str()) {
                        Some("") => Output::Typed,
                        Some(rest) => type_or_copy(rest, &text),
                        None => {
                            tracing::warn!(
                                "final text ({} chars) does not extend the {} chars \
                                 already typed — copying to clipboard instead",
                                text.len(),
                                stream_typed.len()
                            );
                            copy_to_clipboard(&text);
                            Output::Mismatch
                        }
                    }
                } else {
                    type_or_copy(&text, &text)
                };
            }
        }
    }
}

/// How a session's text reached the user.
#[derive(Clone, Copy)]
enum Output {
    Typed,
    Copied,
    /// Final text disagreed with the streamed text already typed.
    Mismatch,
}

/// Type `to_type`; if that fails, put the whole `full` text on the clipboard.
fn type_or_copy(to_type: &str, full: &str) -> Output {
    match type_text(to_type) {
        Ok(()) => Output::Typed,
        Err(e) => {
            tracing::warn!("direct injection failed: {e} — copying to clipboard instead");
            copy_to_clipboard(full);
            Output::Copied
        }
    }
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
