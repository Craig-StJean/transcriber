use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;
use std::time::Duration;

use common::config::{self, AppConfig};
use gtk4::glib;
use libadwaita as adw;

use crate::{daemon, ui};

type Edit = Box<dyn Fn(&mut AppConfig)>;

/// Debounced, merge-on-write config persistence.
///
/// Every widget handler records its change as an *edit* (a closure) rather
/// than mutating a long-lived struct that later gets written wholesale. On
/// save the queued edits are replayed onto a fresh read of `config.json`, so
/// fields changed behind the GUI's back — by the NixOS activation hook, or by
/// hand in an editor — survive instead of being reverted by a stale copy.
#[derive(Clone)]
pub struct AutoSaver {
    inner: Rc<Inner>,
}

struct Inner {
    /// What the widgets read from. Edits are applied here immediately so
    /// handlers that consult other fields (e.g. "is post-processing on?") see
    /// the user's latest choices before the debounced write lands.
    config:  RefCell<AppConfig>,
    pending: RefCell<Vec<Edit>>,
    timer:   RefCell<Option<glib::SourceId>>,
    /// False while `config.json` failed to parse: writing would replace the
    /// user's (probably one-typo-away) file with whatever the GUI holds.
    enabled: Cell<bool>,
    toast:   adw::ToastOverlay,
}

impl AutoSaver {
    pub fn new(cfg: AppConfig, enabled: bool, toast: &adw::ToastOverlay) -> Self {
        Self {
            inner: Rc::new(Inner {
                config:  RefCell::new(cfg),
                pending: RefCell::new(Vec::new()),
                timer:   RefCell::new(None),
                enabled: Cell::new(enabled),
                toast:   toast.clone(),
            }),
        }
    }

    /// Read-only view of the in-memory config.
    pub fn config(&self) -> Ref<'_, AppConfig> {
        self.inner.config.borrow()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.inner.enabled.set(enabled);
    }

    /// Replace the in-memory copy wholesale (after a reset to defaults).
    pub fn replace(&self, cfg: AppConfig) {
        self.inner.pending.borrow_mut().clear();
        *self.inner.config.borrow_mut() = cfg;
    }

    /// Apply `f` now and persist it after a short debounce.
    ///
    /// `f` is `Fn`, not `FnOnce`, because it runs twice: once on the in-memory
    /// copy and again on the fresh read at save time.
    pub fn update(&self, f: impl Fn(&mut AppConfig) + 'static) {
        f(&mut self.inner.config.borrow_mut());
        if !self.inner.enabled.get() {
            return;
        }
        self.inner.pending.borrow_mut().push(Box::new(f));
        self.schedule();
    }

    fn schedule(&self) {
        if let Some(id) = self.inner.timer.borrow_mut().take() {
            id.remove();
        }
        let this = self.clone();
        *self.inner.timer.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(300),
            move || {
                *this.inner.timer.borrow_mut() = None;
                if this.write() {
                    daemon::reload_config(&this.inner.toast);
                }
            },
        ));
    }

    /// Write any pending edits immediately. Called on window close and app
    /// shutdown, where the 300 ms debounce would otherwise never fire and the
    /// last keystrokes would be silently lost.
    pub fn flush(&self) {
        if let Some(id) = self.inner.timer.borrow_mut().take() {
            id.remove();
        }
        if self.write() {
            // The main loop is about to stop, so an async call would never be
            // dispatched; fire-and-forget is the only form that reliably
            // leaves the process.
            daemon::reload_config_no_reply();
        }
    }

    /// Returns true if something was written.
    fn write(&self) -> bool {
        let edits: Vec<Edit> = std::mem::take(&mut *self.inner.pending.borrow_mut());
        if edits.is_empty() || !self.inner.enabled.get() {
            return false;
        }
        match config::update(|cfg| edits.iter().for_each(|e| e(cfg))) {
            Ok(fresh) => {
                *self.inner.config.borrow_mut() = fresh;
                true
            }
            Err(e) => {
                eprintln!("save error: {e:#}");
                // Keep the edits so the next change retries them, rather than
                // dropping everything typed since the last good save.
                let mut pending = self.inner.pending.borrow_mut();
                let newer = std::mem::replace(&mut *pending, edits);
                pending.extend(newer);
                ui::toast(&self.inner.toast, &format!("Failed to save settings: {e}"));
                false
            }
        }
    }
}
