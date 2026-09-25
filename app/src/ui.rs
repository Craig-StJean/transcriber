//! Small widget helpers shared across pages.

use libadwaita as adw;
use adw::prelude::*;

pub fn toast(overlay: &adw::ToastOverlay, title: &str) {
    overlay.add_toast(adw::Toast::builder().title(title).timeout(4).build());
}

/// Every copy action uses this so the confirmation reads the same everywhere.
pub fn copy_to_clipboard(overlay: &adw::ToastOverlay, text: &str) {
    if let Some(display) = gtk4::gdk::Display::default() {
        display.clipboard().set_text(text);
    }
    overlay.add_toast(adw::Toast::builder().title("Copied to clipboard").timeout(2).build());
}

/// Toast with an "Undo" button. `on_undo` runs only if the button is pressed;
/// `on_commit` runs once the toast goes away without it (timeout, dismissal,
/// or being replaced by a newer toast).
pub fn undo_toast(
    overlay: &adw::ToastOverlay,
    title: &str,
    on_undo: impl Fn() + 'static,
    on_commit: impl Fn() + 'static,
) {
    let toast = adw::Toast::builder()
        .title(title)
        .button_label("Undo")
        .timeout(5)
        .build();
    let undone = std::rc::Rc::new(std::cell::Cell::new(false));
    toast.connect_button_clicked({
        let undone = undone.clone();
        move |_| {
            undone.set(true);
            on_undo();
        }
    });
    toast.connect_dismissed(move |_| {
        if !undone.get() {
            on_commit();
        }
    });
    overlay.add_toast(toast);
}

/// Reset a multi-line text buffer to `default`, offering Undo back to what
/// the user had. These buffers hold hand-written prompts and term lists, so a
/// mis-click on "Reset" must not be the end of them.
pub fn reset_buffer_with_undo(
    overlay: &adw::ToastOverlay,
    buffer: &gtk4::TextBuffer,
    default: &str,
    what: &str,
) {
    let previous = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
    if previous == default {
        return;
    }
    buffer.set_text(default);
    let buffer = buffer.clone();
    undo_toast(overlay, &format!("{what} reset to default"), move || buffer.set_text(&previous), || {});
}

/// "5 min ago"-style age of a Unix timestamp; empty for 0 (unknown).
pub fn format_timestamp(ts: i64) -> String {
    if ts == 0 {
        return String::new();
    }
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64))
        .unwrap_or_default()
        .as_secs();
    let plural = |n: u64, unit: &str| format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" });
    match elapsed {
        0..=59       => "just now".into(),
        60..=3599    => format!("{} min ago", elapsed / 60),
        3600..=86399 => format!("{} hr ago", elapsed / 3600),
        s            => plural(s / 86400, "day"),
    }
}
