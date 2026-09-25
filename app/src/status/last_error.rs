//! "Last error" row: the most recent `ErrorOccurred` the daemon emitted while
//! this window was open. The overlay/extension show errors only briefly; this
//! is where a user can read one they missed.

use std::cell::RefCell;
use std::rc::Rc;

use common::dbus::{INTERFACE_NAME, OBJECT_PATH, SERVICE_NAME};
use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;

use crate::ui;

pub struct LastError {
    pub row:      adw::ActionRow,
    icon:         gtk4::Image,
    age:          gtk4::Label,
    /// Message and Unix time it arrived.
    last:         RefCell<Option<(String, i64)>>,
    /// Dropping it unsubscribes.
    subscription: RefCell<Option<gio::SignalSubscription>>,
}

impl LastError {
    pub fn new() -> Rc<Self> {
        let row = adw::ActionRow::builder()
            .title("Last error")
            .use_markup(false)
            .build();
        let icon = gtk4::Image::builder()
            .pixel_size(16)
            .valign(gtk4::Align::Center)
            .icon_name("dialog-error-symbolic")
            .css_classes(vec!["error"])
            .visible(false)
            .build();
        row.add_prefix(&icon);
        let age = gtk4::Label::builder().valign(gtk4::Align::Center).build();
        age.add_css_class("dim-label");
        row.add_suffix(&age);

        let this = Rc::new(LastError {
            row,
            icon,
            age,
            last: RefCell::new(None),
            subscription: RefCell::new(None),
        });
        this.update();
        this.subscribe();

        // Keep "5 min ago" honest while the window is open.
        glib::timeout_add_seconds_local(30, {
            let weak = Rc::downgrade(&this);
            move || {
                let Some(this) = weak.upgrade() else { return glib::ControlFlow::Break };
                this.update();
                glib::ControlFlow::Continue
            }
        });
        this
    }

    fn subscribe(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let Ok(conn) = gio::bus_get_future(gio::BusType::Session).await else { return };
            let Some(this) = weak.upgrade() else { return };
            // A weak reference: the subscription lives inside `this`, so a
            // strong one would keep both alive forever.
            let weak = Rc::downgrade(&this);
            let sub = conn.subscribe_to_signal(
                Some(SERVICE_NAME),
                Some(INTERFACE_NAME),
                Some("ErrorOccurred"),
                Some(OBJECT_PATH),
                None,
                gio::DBusSignalFlags::NONE,
                move |signal| {
                    let Some(this) = weak.upgrade() else { return };
                    let Some((message,)) = signal.parameters.get::<(String,)>() else { return };
                    let now = glib::real_time() / 1_000_000;
                    *this.last.borrow_mut() = Some((message, now));
                    this.update();
                },
            );
            *this.subscription.borrow_mut() = Some(sub);
        });
    }

    /// Stop listening. Called when the window is destroyed.
    pub fn disconnect(&self) {
        self.subscription.borrow_mut().take();
    }

    pub fn update(&self) {
        match &*self.last.borrow() {
            None => {
                self.row.set_subtitle("None since this window opened");
                self.icon.set_visible(false);
                self.age.set_visible(false);
                self.age.set_tooltip_text(None);
            }
            Some((message, at)) => {
                self.row.set_subtitle(message);
                self.icon.set_visible(true);
                self.age.set_label(&ui::format_timestamp(*at));
                self.age.set_visible(true);
                let exact = glib::DateTime::from_unix_local(*at).ok().and_then(|d| d.format("%c").ok());
                self.age.set_tooltip_text(exact.as_deref());
            }
        }
    }
}
