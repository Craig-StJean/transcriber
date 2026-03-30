use std::sync::{Arc, Mutex};

use anyhow::Result;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;

use crate::daemon_proxy::DaemonProxy;

/// Register "toggle-recording" and "cancel-recording" via the XDG GlobalShortcuts
/// portal, then loop forever forwarding activations to daemon method calls.
///
/// On first run the portal shows a one-time approval dialog.  After approval
/// the shortcuts are persisted by the portal across reboots.
///
/// Returns an error if the portal is unavailable (e.g. no xdg-desktop-portal
/// running).  The caller should log the error and continue without hotkeys —
/// the overlay will still respond to daemon DBus signals.
pub async fn run(proxy: DaemonProxy<'static>, current_state: Arc<Mutex<String>>) -> Result<()> {
    let gs = GlobalShortcuts::new()
        .await
        .map_err(|e| anyhow::anyhow!("GlobalShortcuts portal unavailable: {e}"))?;

    let session = gs.create_session().await?;

    // bind_shortcuts returns a Request<BindShortcuts>; call .response() to
    // wait for the portal to confirm (may show user approval dialog first).
    gs.bind_shortcuts(
        &session,
        &[
            NewShortcut::new("toggle-recording", "Toggle Recording"),
            NewShortcut::new("cancel-recording", "Cancel Recording"),
        ],
        None,
    )
    .await?
    .response()?;

    tracing::info!("global shortcuts registered via XDG portal");

    // Each Activated signal carries a single shortcut_id, not a list.
    let mut activated = gs.receive_activated().await?;

    while let Some(event) = activated.next().await {
        match event.shortcut_id() {
            "toggle-recording" => {
                let state = current_state.lock().unwrap().clone();
                let result = match state.as_str() {
                    "Idle" | "Done" | "Error" => proxy.start_recording().await,
                    "Recording" => proxy.stop_recording().await,
                    // Already transcribing — ignore
                    _ => continue,
                };
                if let Err(e) = result {
                    tracing::error!("daemon toggle call failed: {e}");
                }
            }
            "cancel-recording" => {
                if let Err(e) = proxy.cancel().await {
                    tracing::error!("daemon cancel call failed: {e}");
                }
            }
            other => {
                tracing::debug!("unknown shortcut activated: {other}");
            }
        }
    }

    Ok(())
}
