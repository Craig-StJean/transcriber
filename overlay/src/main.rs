mod daemon_proxy;
mod overlay_window;
mod shortcuts;

use std::sync::{Arc, Mutex};

use daemon_proxy::DaemonEvent;
use gtk4::glib;
use gtk4::prelude::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app = gtk4::Application::builder()
        .application_id("org.transcriber.Overlay")
        .build();

    app.connect_activate(|app| {
        let direct_injection = common::config::load()
            .map(|c| c.direct_injection)
            .unwrap_or(false);

        // Check compositor support before doing anything gtk4-layer-shell specific.
        if !gtk4_layer_shell::is_supported() {
            tracing::error!(
                "compositor does not support wlr-layer-shell — \
                 this overlay only works on wlroots-based compositors \
                 (Sway, Hyprland, KDE Plasma 6, …). \
                 On GNOME, use the Shell Extension instead."
            );
        }

        // async-channel bridges the tokio thread ↔ glib executor.
        // Sender is Send + Clone so the tokio thread can use it freely.
        // Receiver is driven by glib::MainContext::spawn_local below.
        let (event_tx, event_rx) = async_channel::unbounded::<DaemonEvent>();

        // Shared current state — written by the signal-streaming tokio task,
        // read by the shortcut activation handler (both in tokio context).
        let current_state: Arc<Mutex<String>> = Arc::new(Mutex::new("Idle".to_string()));

        // Spawn the tokio background thread.
        let state_for_thread = current_state.clone();
        std::thread::spawn({
            let tx = event_tx.clone();
            move || {
                let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
                rt.block_on(run_async(tx, state_for_thread));
            }
        });

        let overlay = overlay_window::OverlayWindow::new(app);
        let overlay_rx = overlay.clone();

        // Drive the event receiver on the glib executor (GTK main thread).
        // async-channel futures are executor-agnostic so they work with glib's
        // spawn_local just as well as with tokio's spawn_local.
        glib::MainContext::default().spawn_local(async move {
            // Cursor into the running streaming transcript — chunks are cumulative,
            // so we type only the new suffix on each emission.
            let mut last_chunk: String = String::new();
            while let Ok(event) = event_rx.recv().await {
                match event {
                    DaemonEvent::StateChanged(ref s) => {
                        if s == "Idle" || s == "Recording" {
                            last_chunk.clear();
                        }
                        overlay_rx.handle_state(s);
                    }
                    DaemonEvent::AudioLevel(level) => {
                        overlay_rx.set_level(level);
                    }
                    DaemonEvent::TranscriptionChunk(ref text) => {
                        // Only streaming + direct_injection produces a usable UX here.
                        // In clipboard mode the final TranscriptionReady wins; skip.
                        if direct_injection && text.starts_with(&last_chunk) {
                            let delta = &text[last_chunk.len()..];
                            if !delta.is_empty() {
                                use enigo::{Enigo, Keyboard, Settings};
                                match Enigo::new(&Settings::default()) {
                                    Ok(mut e) => { let _ = e.text(delta); }
                                    Err(err)  => tracing::warn!("direct inject (chunk) failed: {err}"),
                                }
                            }
                            last_chunk = text.clone();
                        }
                    }
                    DaemonEvent::TranscriptionReady(ref text) => {
                        if direct_injection {
                            // If streaming already typed everything, this is a no-op.
                            // Otherwise (batch path) type the whole thing.
                            let to_type: &str = if text.starts_with(&last_chunk) {
                                &text[last_chunk.len()..]
                            } else {
                                text
                            };
                            if !to_type.is_empty() {
                                use enigo::{Enigo, Keyboard, Settings};
                                match Enigo::new(&Settings::default()) {
                                    Ok(mut e) => { let _ = e.text(to_type); }
                                    Err(err)  => tracing::warn!("direct inject failed: {err}"),
                                }
                            }
                            last_chunk.clear();
                        } else {
                            // Inject transcribed text into the Wayland clipboard via GDK.
                            // Works because this process has an active Wayland display
                            // connection — no need for the wl-clipboard-rs subprocess trick.
                            if let Some(display) = gtk4::gdk::Display::default() {
                                display.clipboard().set_text(text);
                            }
                        }
                    }
                }
            }
        });
    });

    app.run();
}

/// Runs in a dedicated thread under the Tokio runtime.
/// Connects to the daemon, streams signals, and registers global shortcuts.
/// Reconnects automatically after a disconnect.
async fn run_async(
    event_tx: async_channel::Sender<DaemonEvent>,
    current_state: Arc<Mutex<String>>,
) {
    loop {
        tracing::info!("connecting to daemon…");
        match daemon_proxy::connect(event_tx.clone(), current_state.clone()).await {
            Ok(proxy) => {
                tracing::info!("connected to daemon");
                // Register global shortcuts and block until the portal stream ends.
                if let Err(e) = shortcuts::run(proxy, current_state.clone()).await {
                    tracing::warn!("global shortcuts unavailable: {e}");
                    // No hotkeys, but the overlay still works via DBus signals.
                    // Block forever so we don't busy-loop the reconnect.
                    std::future::pending::<()>().await;
                }
            }
            Err(e) => {
                tracing::error!("failed to connect to daemon: {e}");
            }
        }

        // Reset overlay to Idle after a disconnect.
        let _ = event_tx.send(DaemonEvent::StateChanged("Idle".to_string())).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
