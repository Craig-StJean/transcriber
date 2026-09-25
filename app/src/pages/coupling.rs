//! The rules tying streaming, post-processing and direct injection together,
//! kept in one place so each toggle's side effects can be read side by side.
//!
//! - Streaming and post-processing are mutually exclusive. When both are set
//!   the daemon silently prefers post-processing, so the GUI does the same.
//! - Streaming requires direct injection: the daemon refuses to start a
//!   streaming recording without it.
//! - Direct injection is mirrored into the GNOME extension's GSettings, which
//!   decides how the extension presents the result. `config.json` is the
//!   authority (the daemon reads it, and NixOS may declare it there); the
//!   GSettings key follows.

use gtk4::gio;
use libadwaita as adw;
use adw::prelude::*;

use crate::saver::AutoSaver;
use crate::ui;

/// Normalise contradictory settings before any widget is built, so rows start
/// out showing what the daemon will actually do. Returns a note per change
/// for the caller to surface; silently rewriting someone's config is worse
/// than a toast.
pub fn reconcile_startup(saver: &AutoSaver, ext: &Option<gio::Settings>) -> Vec<&'static str> {
    let mut notes = Vec::new();
    let (mut streaming, postprocess, mut direct) = {
        let c = saver.config();
        (c.streaming_enabled, c.postprocess_enabled, c.direct_injection)
    };

    if streaming && postprocess {
        streaming = false;
        saver.update(|c| c.streaming_enabled = false);
        notes.push("Streaming turned off — post-processing was also on and takes priority");
    }
    if streaming && !direct {
        direct = true;
        saver.update(|c| c.direct_injection = true);
        notes.push("Direct injection turned on — streaming requires it");
    }
    if let Some(s) = ext {
        if s.boolean("direct-injection") != direct {
            let _ = s.set_boolean("direct-injection", direct);
        }
    }
    notes
}

pub struct Coupled {
    pub streaming_expander: adw::ExpanderRow,
    pub postprocess_switch: adw::SwitchRow,
    pub direct_inject_row:  adw::SwitchRow,
}

pub fn connect(
    w: Coupled,
    saver: &AutoSaver,
    ext: &Option<gio::Settings>,
    toast: &adw::ToastOverlay,
) {
    let Coupled { streaming_expander, postprocess_switch, direct_inject_row } = w;

    let set_ext_direct = {
        let ext = ext.clone();
        move |v: bool| {
            if let Some(s) = &ext {
                if s.boolean("direct-injection") != v {
                    let _ = s.set_boolean("direct-injection", v);
                }
            }
        }
    };

    streaming_expander.connect_enable_expansion_notify({
        let saver = saver.clone();
        let postprocess_switch = postprocess_switch.clone();
        let direct_inject_row = direct_inject_row.clone();
        let toast = toast.clone();
        let set_ext_direct = set_ext_direct.clone();
        move |row| {
            let enabled = row.enables_expansion();
            if !enabled {
                saver.update(|c| c.streaming_enabled = false);
                return;
            }
            row.set_expanded(true);
            if saver.config().postprocess_enabled {
                postprocess_switch.set_active(false);
                ui::toast(&toast, "Post-processing turned off — it can't be combined with streaming");
            }
            // Written here directly rather than left to the row's notify
            // handler: if the row already shows "on" while the config says
            // "off", `set_active(true)` emits nothing and the config would
            // stay wrong.
            saver.update(|c| {
                c.streaming_enabled = true;
                c.direct_injection = true;
            });
            set_ext_direct(true);
            direct_inject_row.set_active(true);
        }
    });

    postprocess_switch.connect_active_notify({
        let saver = saver.clone();
        let streaming_expander = streaming_expander.clone();
        let toast = toast.clone();
        move |row| {
            let enabled = row.is_active();
            saver.update(move |c| c.postprocess_enabled = enabled);
            if enabled && saver.config().streaming_enabled {
                streaming_expander.set_enable_expansion(false);
                ui::toast(&toast, "Streaming turned off — it can't be combined with post-processing");
            }
        }
    });

    direct_inject_row.connect_active_notify({
        let saver = saver.clone();
        let streaming_expander = streaming_expander.clone();
        let toast = toast.clone();
        move |row| {
            let v = row.is_active();
            saver.update(move |c| c.direct_injection = v);
            set_ext_direct(v);
            if !v && saver.config().streaming_enabled {
                streaming_expander.set_enable_expansion(false);
                ui::toast(&toast, "Streaming turned off — it requires direct injection");
            }
        }
    });

    // Follow changes made outside this window (e.g. by dconf-editor). Routing
    // them through the row keeps the rules above in force.
    if let Some(s) = ext {
        s.connect_changed(Some("direct-injection"), move |s, key| {
            let v = s.boolean(key);
            if direct_inject_row.is_active() != v {
                direct_inject_row.set_active(v);
            }
        });
    }
}
