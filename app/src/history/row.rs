use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;

use super::db::HistoryEntry;
use crate::{daemon, ui};

/// What a row needs from its page.
#[derive(Clone)]
pub struct RowCtx {
    pub toast:     adw::ToastOverlay,
    pub on_delete: Rc<dyn Fn(i64)>,
    /// Re-read the database now instead of waiting for the next poll.
    pub refresh:   Rc<dyn Fn()>,
}

/// A displayed row plus what it was built from, so a refresh can tell
/// whether it needs rebuilding.
pub struct RowHandle {
    pub entry:    HistoryEntry,
    pub row:      adw::ActionRow,
    /// " · 3.2s", filled in off the main thread once the WAV header is read.
    pub duration: RefCell<String>,
}

impl RowHandle {
    pub fn refresh_subtitle(&self) {
        self.row.set_subtitle(&format!(
            "{}{}",
            format_timestamp(self.entry.timestamp),
            self.duration.borrow()
        ));
    }
}

pub fn build(entry: HistoryEntry, ctx: &RowCtx) -> RowHandle {
    let failed = entry.failed();

    let title = if failed {
        let msg = entry.error.as_deref().unwrap_or("unknown error");
        format!("Failed: {}", glib::markup_escape_text(msg))
    } else {
        glib::markup_escape_text(&entry.text).into()
    };

    let row = adw::ActionRow::builder().title(title).build();
    if entry.timestamp > 0 {
        // Relative times are compact but vague; the exact time is one hover away.
        if let Ok(dt) = glib::DateTime::from_unix_local(entry.timestamp) {
            if let Ok(s) = dt.format("%c") {
                row.set_tooltip_text(Some(&s));
            }
        }
    }

    // Unescaped text currently shown in the row. The toggle below flips it
    // between the polished and original transcript; the copy button reads it so
    // it copies whichever version the user is looking at.
    let displayed: Rc<RefCell<String>> = Rc::new(RefCell::new(entry.text.clone()));

    if failed {
        let icon = gtk4::Image::builder()
            .icon_name("dialog-error-symbolic")
            .css_classes(vec!["error"])
            .build();
        row.add_prefix(&icon);
    } else if let Some(raw) = entry.text_original.as_deref().filter(|raw| *raw != entry.text) {
        add_original_toggle(&row, &entry.text, raw, &displayed);
    }

    if !failed {
        let copy_btn = gtk4::Button::builder()
            .icon_name("edit-copy-symbolic")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["flat"])
            .tooltip_text("Copy to clipboard")
            .build();
        copy_btn.connect_clicked({
            let displayed = Rc::clone(&displayed);
            let toast = ctx.toast.clone();
            move |_| ui::copy_to_clipboard(&toast, &displayed.borrow())
        });
        row.add_suffix(&copy_btn);
    }

    if failed && entry.wav_path.is_some() {
        add_retry_button(&row, &entry, ctx);
    }

    if let Some(wav_path) = entry.wav_path.as_deref() {
        // A stat, not a read — cheap enough to do inline.
        if std::path::Path::new(wav_path).exists() {
            add_play_button(&row, wav_path);
        }
    }

    let del_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["flat"])
        .tooltip_text("Delete")
        .build();
    del_btn.connect_clicked({
        let on_delete = Rc::clone(&ctx.on_delete);
        let id = entry.id;
        move |_| on_delete(id)
    });
    row.add_suffix(&del_btn);

    let handle = RowHandle { entry, row, duration: RefCell::new(String::new()) };
    handle.refresh_subtitle();
    handle
}

/// Swaps the visible title between the polished text and the original
/// transcript. Uses a neutral "revert" glyph: the star icons it used before
/// read as "favourite", which this app has no concept of.
fn add_original_toggle(
    row: &adw::ActionRow,
    polished_raw: &str,
    original_raw: &str,
    displayed: &Rc<RefCell<String>>,
) {
    let polished = glib::markup_escape_text(polished_raw).to_string();
    let original = glib::markup_escape_text(original_raw).to_string();
    let polished_raw = polished_raw.to_string();
    let original_raw = original_raw.to_string();

    let toggle = gtk4::ToggleButton::builder()
        .icon_name("document-revert-symbolic")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["flat"])
        .tooltip_text("Show original (unpolished) transcript")
        .build();

    toggle.connect_toggled({
        let row = row.clone();
        let displayed = Rc::clone(displayed);
        move |t| {
            if t.is_active() {
                row.set_title(&original);
                *displayed.borrow_mut() = original_raw.clone();
                t.set_tooltip_text(Some("Show polished transcript"));
            } else {
                row.set_title(&polished);
                *displayed.borrow_mut() = polished_raw.clone();
                t.set_tooltip_text(Some("Show original (unpolished) transcript"));
            }
        }
    });
    row.add_prefix(&toggle);
}

fn add_retry_button(row: &adw::ActionRow, entry: &HistoryEntry, ctx: &RowCtx) {
    let spinner = adw::Spinner::builder()
        .visible(false)
        .valign(gtk4::Align::Center)
        .build();
    let retry_btn = gtk4::Button::builder()
        .label("Retry")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["suggested-action"])
        .build();
    row.add_suffix(&spinner);
    row.add_suffix(&retry_btn);

    let id = entry.id;
    // Ignored by the daemon now (it looks the path up by id) but still part
    // of the method signature.
    let wav = entry.wav_path.clone().unwrap_or_default();
    let ctx = ctx.clone();
    retry_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        spinner.set_visible(true);
        let btn = btn.clone();
        let spinner = spinner.clone();
        let ctx = ctx.clone();
        let params = (id, wav.as_str()).to_variant();
        glib::spawn_future_local(async move {
            let result = daemon::call("RetryTranscription", Some(params), daemon::RETRY_TIMEOUT_MS).await;
            spinner.set_visible(false);
            btn.set_sensitive(true);
            if let Err(e) = result {
                ui::toast(&ctx.toast, &format!("Retry failed: {}", daemon::error_message(&e)));
            }
            // On success the row's content changed in the database; on
            // failure its error message may have. Either way, show it now.
            (ctx.refresh)();
        });
    });
}

fn add_play_button(row: &adw::ActionRow, wav_path: &str) {
    let play_btn = gtk4::Button::builder()
        .icon_name("media-playback-start-symbolic")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["flat"])
        .tooltip_text("Play recording")
        .build();

    let media_file: Rc<RefCell<Option<gtk4::MediaFile>>> = Rc::new(RefCell::new(None));

    let show_stopped = |btn: &gtk4::Button| {
        btn.set_icon_name("media-playback-start-symbolic");
        btn.set_tooltip_text(Some("Play recording"));
    };

    play_btn.connect_clicked({
        let wav_path = wav_path.to_string();
        let media_file = Rc::clone(&media_file);
        move |btn| {
            if let Some(mf) = media_file.borrow_mut().take() {
                mf.set_playing(false);
                show_stopped(btn);
                return;
            }

            let mf = gtk4::MediaFile::for_file(&gio::File::for_path(&wav_path));
            mf.connect_ended_notify({
                let btn = btn.clone();
                let media_file = Rc::clone(&media_file);
                move |mf| {
                    if mf.is_ended() {
                        show_stopped(&btn);
                        *media_file.borrow_mut() = None;
                    }
                }
            });
            mf.play();
            btn.set_icon_name("media-playback-stop-symbolic");
            btn.set_tooltip_text(Some("Stop playback"));
            *media_file.borrow_mut() = Some(mf);
        }
    });

    // Stop audio when the row goes away — hidden by a delete, rebuilt after a
    // retry, or its page switched away — rather than playing on from a
    // control the user can no longer see.
    play_btn.connect_unmap(move |_| {
        if let Some(mf) = media_file.borrow_mut().take() {
            mf.set_playing(false);
        }
    });

    row.add_suffix(&play_btn);
}

pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{secs:.1}s")
    } else {
        let secs = secs as u64;
        format!("{}m {:02}s", secs / 60, secs % 60)
    }
}

pub fn format_timestamp(ts: i64) -> String {
    if ts == 0 {
        return String::new();
    }
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH + Duration::from_secs(ts as u64))
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
