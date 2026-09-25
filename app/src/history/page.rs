use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;

use super::db::{self, HistoryDb};
use super::row::{self, RowCtx, RowHandle};
use crate::saver::AutoSaver;
use crate::{daemon, ui};

#[derive(Clone)]
pub struct HistoryPage {
    inner: Rc<Inner>,
}

struct Inner {
    widget:        gtk4::Box,
    search_bar:    gtk4::SearchBar,
    search_entry:  gtk4::SearchEntry,
    search_toggle: gtk4::ToggleButton,
    clear_btn:     gtk4::Button,
    stack:         gtk4::Stack,
    empty:         adw::StatusPage,
    list_box:      gtk4::ListBox,
    /// Same order as `list_box`'s children: newest first.
    rows:          RefCell<Vec<RowHandle>>,
    /// Deleted in the UI, awaiting the undo toast's timeout before the
    /// daemon is told. Hidden rather than removed so Undo is instant.
    pending:       RefCell<HashSet<i64>>,
    db:            RefCell<HistoryDb>,
    /// Lowercased search text.
    needle:        RefCell<String>,
    toast:         adw::ToastOverlay,
    saver:         AutoSaver,
    hotkey:        Option<String>,
    ctx:           RowCtx,
}

impl HistoryPage {
    pub fn new(toast: &adw::ToastOverlay, saver: &AutoSaver, hotkey: Option<String>) -> Self {
        let inner = Rc::new_cyclic(|weak: &Weak<Inner>| {
            let ctx = RowCtx {
                toast: toast.clone(),
                on_delete: Rc::new({
                    let weak = weak.clone();
                    move |id| {
                        if let Some(inner) = weak.upgrade() {
                            HistoryPage { inner }.delete(id);
                        }
                    }
                }),
                refresh: Rc::new({
                    let weak = weak.clone();
                    move || {
                        if let Some(inner) = weak.upgrade() {
                            HistoryPage { inner }.refresh();
                        }
                    }
                }),
            };
            build_inner(toast, saver, hotkey, ctx)
        });
        let page = HistoryPage { inner };
        page.wire();
        page.inner.db.borrow_mut().changed(); // baseline for the poll
        page.refresh();
        page
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.inner.widget
    }

    /// Search toggle and Clear All, for the header bar.
    pub fn header_buttons(&self) -> (gtk4::ToggleButton, gtk4::Button) {
        (self.inner.search_toggle.clone(), self.inner.clear_btn.clone())
    }

    pub fn start_search(&self) {
        self.inner.search_bar.set_search_mode(true);
        self.inner.search_entry.grab_focus();
    }

    fn wire(&self) {
        let i = &self.inner;

        i.search_toggle
            .bind_property("active", &i.search_bar, "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();
        i.search_bar.connect_search_mode_enabled_notify({
            let entry = i.search_entry.clone();
            move |bar| {
                if !bar.is_search_mode() {
                    entry.set_text("");
                }
            }
        });
        i.search_entry.connect_search_changed({
            let weak = Rc::downgrade(i);
            move |e| {
                let Some(inner) = weak.upgrade() else { return };
                *inner.needle.borrow_mut() = e.text().to_lowercase();
                HistoryPage { inner }.update_visibility();
            }
        });

        i.clear_btn.connect_clicked({
            let weak = Rc::downgrade(i);
            move |btn| {
                if let Some(inner) = weak.upgrade() {
                    HistoryPage { inner }.confirm_clear(btn);
                }
            }
        });

        // Picks up new transcriptions, retries finishing, and the daemon's
        // retention pruning. Cheap: one PRAGMA per tick, and a reload only
        // when something actually committed.
        glib::timeout_add_seconds_local(2, {
            let weak = Rc::downgrade(i);
            move || {
                let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
                let changed = inner.db.borrow_mut().changed();
                if changed {
                    HistoryPage { inner }.refresh();
                }
                glib::ControlFlow::Continue
            }
        });

        // Keep "5 min ago" honest.
        glib::timeout_add_seconds_local(30, {
            let weak = Rc::downgrade(i);
            move || {
                let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
                inner.rows.borrow().iter().for_each(RowHandle::refresh_subtitle);
                glib::ControlFlow::Continue
            }
        });
    }

    /// Bring the list in line with the database, touching only rows that are
    /// new or whose content changed. Rebuilding everything would stop any
    /// playing recording and reset every polished/original toggle each time a
    /// transcription landed.
    pub fn refresh(&self) {
        let i = &self.inner;
        let entries = i.db.borrow_mut().load();
        let by_id: HashMap<i64, &db::HistoryEntry> = entries.iter().map(|e| (e.id, e)).collect();

        let mut rows = i.rows.borrow_mut();
        rows.retain(|h| {
            let keep = by_id.get(&h.entry.id).is_some_and(|e| **e == h.entry);
            if !keep {
                i.list_box.remove(&h.row);
            }
            keep
        });
        // A committed delete is finished once the row is gone from the DB.
        i.pending.borrow_mut().retain(|id| by_id.contains_key(id));

        // Kept rows are already in descending-id order, so walking the fresh
        // list and inserting whatever isn't next in line restores the order.
        let mut kept = std::mem::take(&mut *rows).into_iter().peekable();
        let mut fresh = Vec::new();
        for (pos, entry) in entries.into_iter().enumerate() {
            if kept.peek().is_some_and(|h| h.entry.id == entry.id) {
                rows.push(kept.next().unwrap());
                continue;
            }
            if let Some(wav) = entry.wav_path.clone() {
                fresh.push((entry.id, wav));
            }
            let handle = row::build(entry, &i.ctx);
            i.list_box.insert(&handle.row, pos as i32);
            rows.push(handle);
        }
        drop(rows);

        self.load_durations(fresh);
        self.update_visibility();
    }

    /// Read WAV headers on a worker thread; with hundreds of entries doing it
    /// inline stalled the first paint of the page.
    fn load_durations(&self, wavs: Vec<(i64, String)>) {
        if wavs.is_empty() {
            return;
        }
        let weak = Rc::downgrade(&self.inner);
        glib::spawn_future_local(async move {
            let Ok(durations) = gio::spawn_blocking(move || {
                wavs.into_iter()
                    .filter_map(|(id, wav)| db::wav_duration_secs(&wav).map(|d| (id, d)))
                    .collect::<HashMap<_, _>>()
            })
            .await
            else {
                return;
            };
            let Some(inner) = weak.upgrade() else { return };
            for h in inner.rows.borrow().iter() {
                if let Some(d) = durations.get(&h.entry.id) {
                    *h.duration.borrow_mut() = format!(" \u{00B7} {}", row::format_duration(*d));
                    h.refresh_subtitle();
                }
            }
        });
    }

    fn update_visibility(&self) {
        let i = &self.inner;
        let pending = i.pending.borrow();
        let needle = i.needle.borrow();
        let mut any_entry = false;
        let mut any_visible = false;
        for h in i.rows.borrow().iter() {
            let deleted = pending.contains(&h.entry.id);
            let visible = !deleted && h.entry.matches(&needle);
            h.row.set_visible(visible);
            any_entry |= !deleted;
            any_visible |= visible;
        }

        i.clear_btn.set_sensitive(any_entry);
        let page = if !any_entry {
            self.update_empty_state();
            "empty"
        } else if !any_visible {
            "no-results"
        } else {
            "list"
        };
        i.stack.set_visible_child_name(page);
    }

    fn update_empty_state(&self) {
        let i = &self.inner;
        if !i.saver.config().save_history {
            i.empty.set_title("History Is Off");
            i.empty.set_description(Some(
                "Turn on “Save transcription history” in Settings to keep transcriptions here",
            ));
            return;
        }
        i.empty.set_title("No Transcriptions Yet");
        let how = match &i.hotkey {
            Some(k) => format!("Press {k} to start recording"),
            None => "Press your recording shortcut to start".to_string(),
        };
        i.empty.set_description(Some(&format!("{how}. Transcriptions will appear here.")));
    }

    /// Called when the History tab is shown: the save-history switch may
    /// have changed since, which the empty state describes.
    pub fn on_shown(&self) {
        self.update_visibility();
    }

    fn delete(&self, id: i64) {
        self.inner.pending.borrow_mut().insert(id);
        self.update_visibility();

        let weak = Rc::downgrade(&self.inner);
        let weak2 = weak.clone();
        ui::undo_toast(
            &self.inner.toast,
            "Transcription deleted",
            move || {
                let Some(inner) = weak.upgrade() else { return };
                inner.pending.borrow_mut().remove(&id);
                HistoryPage { inner }.update_visibility();
            },
            move || {
                let Some(inner) = weak2.upgrade() else { return };
                // Gone from `pending` means it was already sent at shutdown
                // or swept up by Clear All.
                if !inner.pending.borrow().contains(&id) {
                    return;
                }
                HistoryPage { inner }.commit_delete(id);
            },
        );
    }

    fn commit_delete(&self, id: i64) {
        let weak = Rc::downgrade(&self.inner);
        glib::spawn_future_local(async move {
            let result = daemon::call(
                "DeleteHistoryEntry",
                Some((id,).to_variant()),
                daemon::SHORT_TIMEOUT_MS,
            )
            .await;
            let Some(inner) = weak.upgrade() else { return };
            let page = HistoryPage { inner };
            if let Err(e) = result {
                page.inner.pending.borrow_mut().remove(&id);
                ui::toast(&page.inner.toast, &format!("Couldn't delete: {}", daemon::error_message(&e)));
            }
            page.refresh();
        });
    }

    /// Send any deletes still waiting on their undo toast. The toast's
    /// dismissal isn't guaranteed to run once the app is quitting, and a
    /// delete the user saw happen must not quietly come back next launch.
    pub fn commit_pending_now(&self) {
        let ids: Vec<i64> = self.inner.pending.borrow_mut().drain().collect();
        for id in ids {
            daemon::send_no_reply("DeleteHistoryEntry", Some((id,).to_variant()));
        }
    }

    fn confirm_clear(&self, parent: &impl IsA<gtk4::Widget>) {
        let dialog = adw::AlertDialog::builder()
            .heading("Clear All History?")
            .body("This will permanently delete all transcriptions and saved recordings.")
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("clear", "Clear All");
        dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let weak = Rc::downgrade(&self.inner);
        dialog.connect_response(Some("clear"), move |_, _| {
            let weak = weak.clone();
            glib::spawn_future_local(async move {
                if let Some(inner) = weak.upgrade() {
                    // Covered by the clear; their toasts must not send a
                    // second, failing delete.
                    inner.pending.borrow_mut().clear();
                }
                let result = daemon::call("ClearHistory", None, daemon::SHORT_TIMEOUT_MS).await;
                let Some(inner) = weak.upgrade() else { return };
                if let Err(e) = result {
                    ui::toast(&inner.toast, &format!("Couldn't clear history: {}", daemon::error_message(&e)));
                }
                HistoryPage { inner }.refresh();
            });
        });
        dialog.present(Some(parent));
    }
}

fn build_inner(
    toast: &adw::ToastOverlay,
    saver: &AutoSaver,
    hotkey: Option<String>,
    ctx: RowCtx,
) -> Inner {
    let stack = gtk4::Stack::new();

    let empty = adw::StatusPage::builder()
        .icon_name("document-open-recent-symbolic")
        .title("No Transcriptions Yet")
        .vexpand(true)
        .build();
    stack.add_named(&empty, Some("empty"));

    let no_results = adw::StatusPage::builder()
        .icon_name("edit-find-symbolic")
        .title("No Results Found")
        .description("Try a different search")
        .vexpand(true)
        .build();
    stack.add_named(&no_results, Some("no-results"));

    let list_box = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(vec!["boxed-list"])
        .valign(gtk4::Align::Start)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    // Deliberately unclamped: history rows use the full window width.
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&list_box)
        .build();
    stack.add_named(&scrolled, Some("list"));

    let search_entry = gtk4::SearchEntry::builder()
        .placeholder_text("Search transcriptions")
        .hexpand(true)
        .build();
    let search_clamp = adw::Clamp::builder().maximum_size(400).child(&search_entry).build();
    let search_bar = gtk4::SearchBar::builder().child(&search_clamp).build();
    search_bar.connect_entry(&search_entry);

    let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    widget.append(&search_bar);
    widget.append(&stack);
    // Type-to-search anywhere on the page.
    search_bar.set_key_capture_widget(Some(&widget));

    let search_toggle = gtk4::ToggleButton::builder()
        .icon_name("edit-find-symbolic")
        .tooltip_text("Search History")
        .build();
    let clear_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Clear All History")
        .build();

    Inner {
        widget,
        search_bar,
        search_entry,
        search_toggle,
        clear_btn,
        stack,
        empty,
        list_box,
        rows: RefCell::new(Vec::new()),
        pending: RefCell::new(HashSet::new()),
        db: RefCell::new(HistoryDb::default()),
        needle: RefCell::new(String::new()),
        toast: toast.clone(),
        saver: saver.clone(),
        hotkey,
        ctx,
    }
}
