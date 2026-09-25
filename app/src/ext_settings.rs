//! Desktop detection and the GNOME Shell extension's GSettings.

use gtk4::gio;
use gtk4::prelude::*;

/// True if the session is GNOME Shell (where the hotkey/VU meter come from the
/// GNOME extension). On Hyprland / Sway / KDE / other wlroots compositors this
/// is false and the standalone overlay binary plays that role instead.
pub fn is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}

/// The extension's settings, if its schema is installed. The schema lives in
/// the extension directory rather than the system schema path, so it has to
/// be loaded from there explicitly.
pub fn load() -> Option<gio::Settings> {
    let schema_dir = dirs::home_dir()?
        .join(".local/share/gnome-shell/extensions/transcriber@local/schemas");
    let source = gio::SettingsSchemaSource::from_directory(
        schema_dir,
        gio::SettingsSchemaSource::default().as_ref(),
        false,
    )
    .ok()?;
    let schema = source.lookup("org.gnome.shell.extensions.transcriber", false)?;
    Some(gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None))
}

/// First accelerator bound to `key`, or `fallback` if unset.
pub fn accel(ext: &Option<gio::Settings>, key: &str, fallback: &str) -> String {
    ext.as_ref()
        .map(|s| s.strv(key))
        .and_then(|v| v.first().map(|g| g.to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// The recording hotkey as a user would read it ("Super+'"), or None where
/// this app can't know it (wlroots, where the compositor owns the binding).
pub fn recording_hotkey_label(ext: &Option<gio::Settings>) -> Option<String> {
    if !is_gnome() {
        return None;
    }
    let accel = accel(ext, "toggle-recording", "<Super>apostrophe");
    let (key, mods) = gtk4::accelerator_parse(&accel)?;
    Some(gtk4::accelerator_get_label(key, mods).to_string())
}
