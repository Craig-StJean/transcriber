//! The "Connections" group: asks the daemon to send a real request to each
//! provider the current configuration uses, and shows one row per answer.
//! Unlike the setup checks this goes over the network (and may cost a
//! request), so it only runs when the Status page is shown or on demand.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;

use crate::daemon;

/// The daemon tests providers one after another with its own timeouts;
/// ~10 s is typical, so leave generous room before calling it a failure.
const TEST_TIMEOUT_MS: i32 = 30_000;

struct Outcome {
    name:   String,
    ok:     bool,
    detail: String,
}

pub struct Connections {
    pub group:  adw::PreferencesGroup,
    spinner:    adw::Spinner,
    button:     gtk4::Button,
    rows:       RefCell<Vec<adw::ActionRow>>,
    running:    Cell<bool>,
}

impl Connections {
    pub fn new() -> Rc<Self> {
        let group = adw::PreferencesGroup::builder()
            .title("Connections")
            .description("Sends a test request to each provider in use")
            .build();

        let spinner = adw::Spinner::builder()
            .visible(false)
            .valign(gtk4::Align::Center)
            .build();
        let button = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["flat"])
            .tooltip_text("Test Connections")
            .build();
        let suffix = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        suffix.append(&spinner);
        suffix.append(&button);
        group.set_header_suffix(Some(&suffix));

        // Replaced by the first result, normally within seconds of the page
        // being shown.
        let placeholder = adw::ActionRow::builder().title("Not tested yet").build();
        group.add(&placeholder);

        let this = Rc::new(Connections {
            group,
            spinner,
            button,
            rows: RefCell::new(vec![placeholder]),
            running: Cell::new(false),
        });
        this.button.connect_clicked({
            let weak = Rc::downgrade(&this);
            move |_| {
                if let Some(this) = weak.upgrade() {
                    this.run();
                }
            }
        });
        this
    }

    /// Start a test unless one is already in flight — each run hits every
    /// provider, so overlapping runs would only duplicate requests.
    pub fn run(self: &Rc<Self>) {
        if self.running.replace(true) {
            return;
        }
        self.spinner.set_visible(true);
        self.button.set_sensitive(false);
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let result = daemon::call("TestConnections", None, TEST_TIMEOUT_MS).await;
            let Some(this) = weak.upgrade() else { return };
            this.running.set(false);
            this.spinner.set_visible(false);
            this.button.set_sensitive(true);
            this.show(match result {
                Ok(v) => parse(&v),
                Err(e) => vec![call_failure(&e)],
            });
        });
    }

    fn show(&self, outcomes: Vec<Outcome>) {
        let mut rows = self.rows.borrow_mut();
        for row in rows.drain(..) {
            self.group.remove(&row);
        }
        for o in outcomes {
            // Details are provider error bodies; never parse them as markup.
            let row = adw::ActionRow::builder()
                .title(&o.name)
                .subtitle(&o.detail)
                .use_markup(false)
                .build();
            let icon = gtk4::Image::builder()
                .pixel_size(16)
                .valign(gtk4::Align::Center)
                .icon_name(if o.ok { "object-select-symbolic" } else { "dialog-error-symbolic" })
                .css_classes(vec![if o.ok { "success" } else { "error" }])
                .build();
            row.add_prefix(&icon);
            self.group.add(&row);
            rows.push(row);
        }
    }
}

/// `TestConnections` returns `s`: a JSON array of `{name, ok, detail}`.
fn parse(reply: &glib::Variant) -> Vec<Outcome> {
    let Some((json,)) = reply.get::<(String,)>() else {
        return vec![failure("Unexpected reply", "The daemon's answer wasn't in the expected format")];
    };
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str(&json) else {
        return vec![failure("Unexpected reply", "The daemon's answer wasn't valid JSON")];
    };
    if items.is_empty() {
        return vec![Outcome {
            name:   "Nothing to test".into(),
            ok:     true,
            detail: "No providers are configured".into(),
        }];
    }
    items
        .iter()
        .map(|item| {
            let s = |k: &str| item.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let ok = item.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let detail = s("detail");
            Outcome {
                name: s("name"),
                ok,
                detail: if !detail.is_empty() {
                    detail
                } else if ok {
                    "Connected".into()
                } else {
                    "Failed".into()
                },
            }
        })
        .collect()
}

fn failure(name: &str, detail: &str) -> Outcome {
    Outcome { name: name.into(), ok: false, detail: detail.into() }
}

fn call_failure(e: &glib::Error) -> Outcome {
    if e.matches(gio::DBusError::UnknownMethod) {
        failure("Daemon out of date", "This version of the daemon can't test connections — update it")
    } else if e.matches(gio::DBusError::ServiceUnknown) || e.matches(gio::DBusError::NameHasNoOwner) {
        failure("Daemon unavailable", "Start the daemon to test connections")
    } else if e.matches(gio::IOErrorEnum::TimedOut) || e.matches(gio::DBusError::NoReply) {
        failure("Test timed out", "The daemon didn't answer in time")
    } else {
        failure("Test failed", &daemon::error_message(e))
    }
}
