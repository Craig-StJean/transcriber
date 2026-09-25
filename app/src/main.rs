mod daemon;
mod ext_settings;
mod history;
mod pages;
mod saver;
mod status;
mod ui;

use common::config::{self, AppConfig};
use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;

use history::HistoryPage;
use pages::coupling;
use saver::AutoSaver;

fn main() -> glib::ExitCode {
    adw::init().expect("failed to initialise libadwaita");
    let app = adw::Application::builder()
        .application_id("org.transcriber.Settings")
        .build();
    app.connect_activate(|app| {
        // Launching the app again activates this instance. Building a second
        // window would give two AutoSavers each replaying edits over the
        // other's, and double every poll timer.
        if let Some(w) = app.windows().first() {
            w.present();
            return;
        }
        build_ui(app);
    });
    app.run()
}

fn build_shortcuts_dialog(ext: &Option<gio::Settings>) -> adw::ShortcutsDialog {
    let dialog = adw::ShortcutsDialog::new();

    let recording = adw::ShortcutsSection::new(Some("Recording"));
    recording.add(adw::ShortcutsItem::new(
        "Toggle Recording",
        &ext_settings::accel(ext, "toggle-recording", "<Super>apostrophe"),
    ));
    recording.add(adw::ShortcutsItem::new(
        "Cancel Recording",
        &ext_settings::accel(ext, "cancel-recording", "Escape"),
    ));
    if let Some(accel) = ext_settings::optional_accel(ext, "repaste-last") {
        // An empty accelerator means unbound; see `label_unbound` below.
        recording.add(adw::ShortcutsItem::new("Re-paste Last Transcription", &accel.unwrap_or_default()));
    }
    dialog.add(recording);

    let app_section = adw::ShortcutsSection::new(Some("Application"));
    app_section.add(adw::ShortcutsItem::new("Settings",        "<Control>1"));
    app_section.add(adw::ShortcutsItem::new("Post-Processing", "<Control>2"));
    app_section.add(adw::ShortcutsItem::new("History",         "<Control>3"));
    app_section.add(adw::ShortcutsItem::new("Status",          "<Control>4"));
    app_section.add(adw::ShortcutsItem::new("Search History",  "<Control>f"));
    app_section.add(adw::ShortcutsItem::new("Keyboard Shortcuts", "<Control>question"));
    app_section.add(adw::ShortcutsItem::new("Quit", "<Control>q"));
    dialog.add(app_section);

    // Unbound shortcuts would read "No Shortcut"; say "Disabled" instead.
    // `ShortcutsItem` doesn't expose its label, so find it once built.
    dialog.connect_map(|d| label_unbound(d.upcast_ref()));
    dialog
}

fn label_unbound(w: &gtk4::Widget) {
    if let Some(label) = w.downcast_ref::<adw::ShortcutLabel>() {
        label.set_disabled_text("Disabled");
        return;
    }
    let mut child = w.first_child();
    while let Some(c) = child {
        label_unbound(&c);
        child = c.next_sibling();
    }
}

fn build_ui(app: &adw::Application) {
    let toast_overlay = adw::ToastOverlay::new();

    // A config that fails to parse must not be papered over with defaults:
    // the first autosave would then overwrite the user's file (keys and all)
    // with those defaults. Show the error, keep saving off, and make
    // resetting an explicit choice.
    let (cfg, load_error) = match config::load() {
        Ok(cfg) => (cfg, None),
        Err(e) => (AppConfig::default(), Some(format!("{e:#}"))),
    };
    let saver = AutoSaver::new(cfg, load_error.is_none(), &toast_overlay);
    let ext_settings = ext_settings::load();
    let startup_notes = if load_error.is_none() {
        coupling::reconcile_startup(&saver, &ext_settings)
    } else {
        Vec::new()
    };

    // ── Pages ───────────────────────────────────────────────────────────────
    let view_stack = adw::ViewStack::new();

    let settings = pages::settings::build(&saver, &ext_settings, &toast_overlay);
    view_stack.add_titled_with_icon(
        &settings.page,
        Some("settings"),
        "Settings",
        "preferences-system-symbolic",
    );

    let (postprocess_page, postprocess_switch) = pages::postprocess::build(&saver, &toast_overlay);
    view_stack.add_titled_with_icon(
        &postprocess_page,
        Some("postprocess"),
        "Post-Processing",
        "applications-utilities-symbolic",
    );

    let settings_page = settings.page.clone();
    coupling::connect(
        coupling::Coupled {
            streaming_expander: settings.streaming_expander,
            postprocess_switch,
            direct_inject_row:  settings.direct_inject_row,
        },
        &saver,
        &ext_settings,
        &toast_overlay,
    );

    let history = HistoryPage::new(
        &toast_overlay,
        &saver,
        ext_settings::recording_hotkey_label(&ext_settings),
    );
    view_stack.add_titled_with_icon(
        history.widget(),
        Some("history"),
        "History",
        "document-open-recent-symbolic",
    );

    let status = status::build(&toast_overlay, &view_stack);
    let status_refresh = status.refresh;
    view_stack.add_titled_with_icon(
        &status.page,
        Some("status"),
        "Status",
        "emblem-system-symbolic",
    );

    // ── Header bar ──────────────────────────────────────────────────────────
    let switcher = adw::InlineViewSwitcher::new();
    switcher.set_stack(Some(&view_stack));

    let menu = gio::Menu::new();
    menu.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
    menu.append(Some("About Transcriber"), Some("app.about"));
    let menu_btn = gtk4::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .tooltip_text("Main Menu")
        .build();
    menu_btn.add_css_class("flat");

    let (search_btn, clear_btn) = history.header_buttons();
    search_btn.set_visible(false);
    clear_btn.set_visible(false);

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&switcher));
    header.pack_start(&search_btn);
    header.pack_end(&menu_btn);
    header.pack_end(&clear_btn);

    // ── Broken-config banner ────────────────────────────────────────────────
    let banner = adw::Banner::builder()
        .button_label("Reset to Defaults")
        .revealed(load_error.is_some())
        .build();
    if let Some(e) = &load_error {
        banner.set_title(&format!(
            "config.json could not be read, so changes won't be saved: {}",
            glib::markup_escape_text(e)
        ));
        // Editing would appear to work and then be silently discarded.
        settings_page.set_sensitive(false);
        postprocess_page.set_sensitive(false);
    }

    // ── Window ──────────────────────────────────────────────────────────────
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.add_top_bar(&banner);
    toolbar_view.set_content(Some(&view_stack));
    toast_overlay.set_child(Some(&toolbar_view));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Transcriber")
        .default_width(640)
        .default_height(580)
        .width_request(360)
        .content(&toast_overlay)
        .build();

    banner.connect_button_clicked({
        let window = window.clone();
        let saver = saver.clone();
        let toast = toast_overlay.clone();
        let pages = [settings_page.clone(), postprocess_page.clone()];
        move |banner| confirm_reset(&window, banner, &saver, &toast, &pages)
    });

    // Flush on both exits: closing the window, and app.quit (Ctrl+Q), which
    // skips close-request entirely. Both calls are idempotent.
    window.connect_close_request({
        let saver = saver.clone();
        let history = history.clone();
        move |_| {
            saver.flush();
            history.commit_pending_now();
            glib::Propagation::Proceed
        }
    });
    window.connect_destroy({
        let last_error = status.last_error;
        move |_| last_error.disconnect()
    });
    app.connect_shutdown({
        let saver = saver.clone();
        let history = history.clone();
        move |_| {
            saver.flush();
            history.commit_pending_now();
        }
    });

    // ── Actions ─────────────────────────────────────────────────────────────
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate({
        let w = window.clone();
        move |_, _| {
            adw::AboutDialog::builder()
                .application_name("Transcriber")
                .application_icon("audio-input-microphone-symbolic")
                .developer_name("Craig")
                .version(env!("CARGO_PKG_VERSION"))
                .build()
                .present(Some(&w));
        }
    });
    app.add_action(&about_action);

    let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
    shortcuts_action.connect_activate({
        let w = window.clone();
        let ext = ext_settings.clone();
        move |_, _| build_shortcuts_dialog(&ext).present(Some(&w))
    });
    app.add_action(&shortcuts_action);
    app.set_accels_for_action("app.shortcuts", &["<Control>question", "<Control>F1"]);

    let quit_action = gio::SimpleAction::new("quit", None);
    quit_action.connect_activate({
        let a = app.clone();
        move |_, _| a.quit()
    });
    app.add_action(&quit_action);
    app.set_accels_for_action("app.quit", &["<Control>q"]);

    for (i, name) in ["settings", "postprocess", "history", "status"].iter().enumerate() {
        let action = gio::SimpleAction::new(&format!("go-{name}"), None);
        action.connect_activate({
            let s = view_stack.clone();
            let n = name.to_string();
            move |_, _| s.set_visible_child_name(&n)
        });
        window.add_action(&action);
        app.set_accels_for_action(&format!("win.go-{name}"), &[&format!("<Control>{}", i + 1)]);
    }

    let search_action = gio::SimpleAction::new("search", None);
    search_action.connect_activate({
        let s = view_stack.clone();
        let history = history.clone();
        move |_, _| {
            s.set_visible_child_name("history");
            history.start_search();
        }
    });
    window.add_action(&search_action);
    app.set_accels_for_action("win.search", &["<Control>f"]);

    // ── Page visibility logic ───────────────────────────────────────────────
    view_stack.connect_visible_child_name_notify({
        let history = history.clone();
        move |stack| {
            let name = stack.visible_child_name();
            let name = name.as_deref();
            let on_history = name == Some("history");
            clear_btn.set_visible(on_history);
            search_btn.set_visible(on_history);
            if on_history {
                history.on_shown();
            }
            if name == Some("status") {
                status_refresh();
            }
        }
    });

    window.present();

    for note in startup_notes {
        ui::toast(&toast_overlay, note);
    }
}

fn confirm_reset(
    window: &adw::ApplicationWindow,
    banner: &adw::Banner,
    saver: &AutoSaver,
    toast: &adw::ToastOverlay,
    pages: &[adw::PreferencesPage; 2],
) {
    let dialog = adw::AlertDialog::builder()
        .heading("Reset Settings?")
        .body(
            "All settings, including API keys, return to their defaults. \
             The unreadable file is kept as config.json.broken.",
        )
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("reset", "Reset");
    dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let banner = banner.clone();
    let saver = saver.clone();
    let toast = toast.clone();
    let pages = pages.clone();
    dialog.connect_response(Some("reset"), move |_, _| {
        let path = config::config_path();
        // Keep the user's file: it probably holds their keys and is likely
        // one typo from valid.
        if let Err(e) = std::fs::copy(&path, path.with_extension("json.broken")) {
            ui::toast(&toast, &format!("Couldn't back up config.json: {e}"));
            return;
        }
        let defaults = AppConfig::default();
        if let Err(e) = config::save(&defaults) {
            ui::toast(&toast, &format!("Couldn't reset config.json: {e}"));
            return;
        }
        // The widgets were built from these same defaults, so they already
        // match what is now on disk.
        saver.replace(defaults);
        saver.set_enabled(true);
        pages.iter().for_each(|p| p.set_sensitive(true));
        banner.set_revealed(false);
        daemon::reload_config(&toast);
        ui::toast(&toast, "Settings reset to defaults");
    });
    dialog.present(Some(window));
}
