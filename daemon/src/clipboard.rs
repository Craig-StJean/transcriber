use anyhow::Result;

/// Copy `text` into the Wayland clipboard.
///
/// `wl-clipboard-rs` forks a child process that holds the selection data
/// until another client claims it.  This requires `WAYLAND_DISPLAY` and
/// `XDG_RUNTIME_DIR` to be set — ensured by the systemd service file via
/// `ExecStartPre=systemctl --user import-environment ...`.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    use wl_clipboard_rs::copy::{MimeType, Options, Source};

    let opts = Options::new();
    opts.copy(
        Source::Bytes(text.as_bytes().into()),
        MimeType::Text,
    )?;

    tracing::debug!("copied {} chars to clipboard", text.len());
    Ok(())
}
