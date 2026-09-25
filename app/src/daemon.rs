//! Thin async wrappers over the daemon's DBus interface.

use common::dbus::{INTERFACE_NAME, OBJECT_PATH, SERVICE_NAME};
use gtk4::{gio, glib};
use libadwaita as adw;

use crate::ui;

/// Default timeout for quick bookkeeping calls.
pub const SHORT_TIMEOUT_MS: i32 = 5_000;

/// `RetryTranscription` re-uploads the WAV and now runs post-processing too;
/// a long recording on a slow provider can legitimately take minutes.
pub const RETRY_TIMEOUT_MS: i32 = 5 * 60 * 1_000;

pub async fn call(
    method: &str,
    params: Option<glib::Variant>,
    timeout_ms: i32,
) -> Result<glib::Variant, glib::Error> {
    let conn = gio::bus_get_future(gio::BusType::Session).await?;
    conn.call_future(
        Some(SERVICE_NAME),
        OBJECT_PATH,
        INTERFACE_NAME,
        method,
        params.as_ref(),
        None,
        gio::DBusCallFlags::NONE,
        timeout_ms,
    )
    .await
}

/// Human-readable message for a failed call. Remote errors arrive as
/// "GDBus.Error:org.freedesktop.DBus.Error.Failed: <text>"; only `<text>` is
/// meaningful to a user.
pub fn error_message(e: &glib::Error) -> String {
    let mut e = e.clone();
    gio::DBusError::strip_remote_error(&mut e);
    e.message().to_string()
}

/// The daemon isn't installed/activatable. Not worth a toast on every save —
/// the Status page is where that gets reported.
fn is_daemon_absent(e: &glib::Error) -> bool {
    e.matches(gio::DBusError::ServiceUnknown) || e.matches(gio::DBusError::NameHasNoOwner)
}

/// Ask the daemon to re-read `config.json`, toasting if it refuses. Without
/// the toast a daemon that rejects the new file keeps running on the old
/// settings while the GUI looks like it saved successfully.
pub fn reload_config(toast: &adw::ToastOverlay) {
    let toast = toast.clone();
    glib::spawn_future_local(async move {
        if let Err(e) = call("ReloadConfig", None, SHORT_TIMEOUT_MS).await {
            if !is_daemon_absent(&e) {
                ui::toast(
                    &toast,
                    &format!("Saved, but the daemon did not reload: {}", error_message(&e)),
                );
            }
        }
    });
}

/// Fire-and-forget `ReloadConfig` for shutdown paths, where no reply could
/// be processed anyway.
pub fn reload_config_no_reply() {
    send_no_reply("ReloadConfig", None);
}

/// Send a method call without waiting for (or wanting) a reply, flushing it
/// onto the socket before returning so it survives process exit.
pub fn send_no_reply(method: &str, params: Option<glib::Variant>) {
    let Ok(conn) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return;
    };
    let msg = gio::DBusMessage::new_method_call(
        Some(SERVICE_NAME),
        OBJECT_PATH,
        Some(INTERFACE_NAME),
        method,
    );
    if let Some(p) = params {
        msg.set_body(&p);
    }
    msg.set_flags(gio::DBusMessageFlags::NO_REPLY_EXPECTED);
    if conn.send_message(&msg, gio::DBusSendMessageFlags::NONE).is_ok() {
        let _ = conn.flush_sync(gio::Cancellable::NONE);
    }
}
