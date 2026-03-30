use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use libadwaita as adw;
use adw::prelude::*;
use gtk4::{gio, glib};
use rusqlite::{params, Connection};
use common::config::{self, AppConfig};

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    adw::init().expect("failed to initialise libadwaita");
    let app = adw::Application::builder()
        .application_id("org.transcriber.Settings")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    let cfg = config::load().unwrap_or_default();

    let toast_overlay = adw::ToastOverlay::new();
    let view_stack    = adw::ViewStack::new();

    view_stack.add_titled_with_icon(
        &build_settings_page(&cfg, &toast_overlay),
        Some("settings"), "Settings", "preferences-system-symbolic",
    );
    view_stack.add_titled_with_icon(
        &build_history_page(),
        Some("history"), "History", "document-open-recent-symbolic",
    );
    let status_page = build_status_page(&view_stack);
    view_stack.add_titled_with_icon(
        &status_page,
        Some("status"), "Status", "emblem-system-symbolic",
    );

    let switcher = adw::InlineViewSwitcher::new();
    switcher.set_stack(Some(&view_stack));

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&switcher));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&view_stack));

    toast_overlay.set_child(Some(&toolbar_view));

    adw::ApplicationWindow::builder()
        .application(app)
        .title("Voice Transcriber")
        .default_width(640)
        .default_height(580)
        .content(&toast_overlay)
        .build()
        .present();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn provider_idx(provider: &str) -> u32 {
    match provider {
        "cohere" => 1,
        "custom" => 2,
        _        => 0,
    }
}

fn provider_str(idx: u32) -> &'static str {
    match idx {
        1 => "cohere",
        2 => "custom",
        _ => "groq",
    }
}

// ── Settings page ─────────────────────────────────────────────────────────────

fn build_settings_page(cfg: &AppConfig, toast_overlay: &adw::ToastOverlay) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();

    // ── Provider selector ────────────────────────────────────────────────────
    let provider_group = adw::PreferencesGroup::builder()
        .title("API Provider")
        .build();

    let provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Groq", "Cohere", "Custom"]))
        .build();
    provider_row.set_selected(provider_idx(&cfg.provider));
    provider_group.add(&provider_row);

    // ── Groq settings ────────────────────────────────────────────────────────
    let groq_group = adw::PreferencesGroup::builder()
        .title("Groq")
        .description("api.groq.com — OpenAI-compatible Whisper endpoint")
        .build();

    let groq_key_row = adw::PasswordEntryRow::builder()
        .title("Groq API Key")
        .text(&cfg.groq_api_key)
        .build();
    let groq_model_row = adw::EntryRow::builder()
        .title("Model")
        .text(&cfg.groq_model)
        .build();
    groq_group.add(&groq_key_row);
    groq_group.add(&groq_model_row);

    // ── Cohere settings ───────────────────────────────────────────────────────
    let cohere_group = adw::PreferencesGroup::builder()
        .title("Cohere")
        .description("api.cohere.com — Cohere Transcribe endpoint")
        .build();

    let cohere_key_row = adw::PasswordEntryRow::builder()
        .title("Cohere API Key")
        .text(&cfg.cohere_api_key)
        .build();
    let cohere_model_row = adw::EntryRow::builder()
        .title("Model")
        .text(&cfg.cohere_model)
        .build();
    cohere_group.add(&cohere_key_row);
    cohere_group.add(&cohere_model_row);

    // ── Custom endpoint settings ──────────────────────────────────────────────
    let custom_group = adw::PreferencesGroup::builder()
        .title("Custom Endpoint")
        .description("Any OpenAI-compatible /audio/transcriptions endpoint")
        .build();

    let custom_url_row = adw::EntryRow::builder()
        .title("API URL")
        .text(&cfg.custom_api_url)
        .build();
    let custom_key_row = adw::PasswordEntryRow::builder()
        .title("API Key")
        .text(&cfg.custom_api_key)
        .build();
    let custom_model_row = adw::EntryRow::builder()
        .title("Model")
        .text(&cfg.custom_model)
        .build();
    custom_group.add(&custom_url_row);
    custom_group.add(&custom_key_row);
    custom_group.add(&custom_model_row);

    // Set initial group visibility based on stored provider
    let initial_idx = provider_idx(&cfg.provider);
    groq_group.set_visible(initial_idx == 0);
    cohere_group.set_visible(initial_idx == 1);
    custom_group.set_visible(initial_idx == 2);

    provider_row.connect_selected_notify({
        let groq_group   = groq_group.clone();
        let cohere_group = cohere_group.clone();
        let custom_group = custom_group.clone();
        move |row| {
            match row.selected() {
                0 => { groq_group.set_visible(true);  cohere_group.set_visible(false); custom_group.set_visible(false); }
                1 => { groq_group.set_visible(false); cohere_group.set_visible(true);  custom_group.set_visible(false); }
                _ => { groq_group.set_visible(false); cohere_group.set_visible(false); custom_group.set_visible(true);  }
            }
        }
    });

    // ── Shared transcription settings ─────────────────────────────────────────
    let transcription_group = adw::PreferencesGroup::builder()
        .title("Transcription")
        .build();

    let lang_row = adw::EntryRow::builder()
        .title("Language (BCP-47)")
        .text(cfg.language.as_deref().unwrap_or(""))
        .build();
    transcription_group.add(&lang_row);

    // ── Audio ─────────────────────────────────────────────────────────────────
    let audio_group = adw::PreferencesGroup::builder().title("Audio").build();

    let adj = gtk4::Adjustment::builder()
        .lower(8000.0).upper(48000.0)
        .step_increment(1000.0).page_increment(8000.0)
        .value(cfg.sample_rate as f64)
        .build();
    let rate_row = adw::SpinRow::builder()
        .title("Sample Rate (Hz)")
        .subtitle("16000 Hz recommended for Whisper")
        .adjustment(&adj)
        .build();
    audio_group.add(&rate_row);

    // ── Behaviour (reads extension GSettings + local save_history setting) ────
    let behaviour_group = adw::PreferencesGroup::builder()
        .title("Behaviour")
        .build();

    let paste_row = adw::SwitchRow::builder()
        .title("Auto-paste after transcription")
        .subtitle("Pastes into the focused window (Ctrl+Shift+V for terminals, Ctrl+V elsewhere)")
        .build();

    let ext_settings: Option<gio::Settings> = (|| {
        let schema_dir = dirs::home_dir()?
            .join(".local/share/gnome-shell/extensions/voice-transcriber@local/schemas");
        let source = gio::SettingsSchemaSource::from_directory(
            schema_dir, gio::SettingsSchemaSource::default().as_ref(), false,
        ).ok()?;
        let schema = source.lookup("org.gnome.shell.extensions.voice-transcriber", false)?;
        Some(gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None))
    })();

    if let Some(ref s) = ext_settings {
        paste_row.set_active(s.boolean("auto-paste"));
        paste_row.connect_active_notify({
            let s = s.clone();
            move |row| { let _ = s.set_boolean("auto-paste", row.is_active()); }
        });
    } else {
        paste_row.set_sensitive(false);
        paste_row.set_subtitle("Extension not installed — setting unavailable");
    }

    let history_row = adw::SwitchRow::builder()
        .title("Save transcription history")
        .subtitle("Record WAV files and transcription results to disk")
        .active(cfg.save_history)
        .build();

    let vad_row = adw::SwitchRow::builder()
        .title("Auto-stop on silence")
        .subtitle("Automatically stop recording after ~1.5 seconds of silence")
        .active(cfg.vad_enabled)
        .build();

    let direct_inject_row = adw::SwitchRow::builder()
        .title("Direct text injection")
        .subtitle("Type text directly into the focused window (no clipboard)")
        .active(cfg.direct_injection)
        .build();

    if let Some(ref s) = ext_settings {
        if s.boolean("direct-injection") { direct_inject_row.set_active(true); }
        direct_inject_row.connect_active_notify({
            let s = s.clone();
            move |row| { let _ = s.set_boolean("direct-injection", row.is_active()); }
        });
    }

    behaviour_group.add(&paste_row);
    behaviour_group.add(&history_row);
    behaviour_group.add(&vad_row);
    behaviour_group.add(&direct_inject_row);

    // ── Save button ───────────────────────────────────────────────────────────
    let actions_group = adw::PreferencesGroup::new();
    let save_row = adw::ButtonRow::builder()
        .title("Save Settings")
        .start_icon_name("document-save-symbolic")
        .sensitive(false)
        .build();
    actions_group.add(&save_row);

    // ── Assemble page ─────────────────────────────────────────────────────────
    page.add(&provider_group);
    page.add(&groq_group);
    page.add(&cohere_group);
    page.add(&custom_group);
    page.add(&transcription_group);
    page.add(&audio_group);
    page.add(&behaviour_group);
    page.add(&actions_group);

    // ── Dirty tracking ────────────────────────────────────────────────────────
    let last_saved = Rc::new(RefCell::new(cfg.clone()));

    let check_dirty: Rc<dyn Fn()> = {
        let last_saved       = Rc::clone(&last_saved);
        let provider_row     = provider_row.clone();
        let groq_key_row     = groq_key_row.clone();
        let groq_model_row   = groq_model_row.clone();
        let cohere_key_row   = cohere_key_row.clone();
        let cohere_model_row = cohere_model_row.clone();
        let custom_url_row   = custom_url_row.clone();
        let custom_key_row   = custom_key_row.clone();
        let custom_model_row = custom_model_row.clone();
        let lang_row         = lang_row.clone();
        let rate_row         = rate_row.clone();
        let history_row         = history_row.clone();
        let vad_row             = vad_row.clone();
        let direct_inject_row   = direct_inject_row.clone();
        let save_row            = save_row.clone();
        Rc::new(move || {
            let s = last_saved.borrow();
            let dirty =
                provider_str(provider_row.selected()) != s.provider.as_str()
                || groq_key_row.text()             != s.groq_api_key.as_str()
                || groq_model_row.text()           != s.groq_model.as_str()
                || cohere_key_row.text()           != s.cohere_api_key.as_str()
                || cohere_model_row.text()         != s.cohere_model.as_str()
                || custom_url_row.text()           != s.custom_api_url.as_str()
                || custom_key_row.text()           != s.custom_api_key.as_str()
                || custom_model_row.text()         != s.custom_model.as_str()
                || lang_row.text()                 != s.language.as_deref().unwrap_or("")
                || rate_row.value() as u32         != s.sample_rate
                || history_row.is_active()         != s.save_history
                || vad_row.is_active()             != s.vad_enabled
                || direct_inject_row.is_active()   != s.direct_injection;
            save_row.set_sensitive(dirty);
        })
    };

    provider_row.connect_selected_notify({    let c = Rc::clone(&check_dirty); move |_| c() });
    groq_key_row.connect_changed({            let c = Rc::clone(&check_dirty); move |_| c() });
    groq_model_row.connect_changed({          let c = Rc::clone(&check_dirty); move |_| c() });
    cohere_key_row.connect_changed({          let c = Rc::clone(&check_dirty); move |_| c() });
    cohere_model_row.connect_changed({        let c = Rc::clone(&check_dirty); move |_| c() });
    custom_url_row.connect_changed({          let c = Rc::clone(&check_dirty); move |_| c() });
    custom_key_row.connect_changed({          let c = Rc::clone(&check_dirty); move |_| c() });
    custom_model_row.connect_changed({        let c = Rc::clone(&check_dirty); move |_| c() });
    lang_row.connect_changed({                let c = Rc::clone(&check_dirty); move |_| c() });
    rate_row.connect_value_notify({           let c = Rc::clone(&check_dirty); move |_| c() });
    history_row.connect_active_notify({          let c = Rc::clone(&check_dirty); move |_| c() });
    vad_row.connect_active_notify({              let c = Rc::clone(&check_dirty); move |_| c() });
    direct_inject_row.connect_active_notify({    let c = Rc::clone(&check_dirty); move |_| c() });

    // ── Save handler ──────────────────────────────────────────────────────────
    save_row.connect_activated({
        let provider_row     = provider_row.clone();
        let groq_key_row     = groq_key_row.clone();
        let groq_model_row   = groq_model_row.clone();
        let cohere_key_row   = cohere_key_row.clone();
        let cohere_model_row = cohere_model_row.clone();
        let custom_url_row   = custom_url_row.clone();
        let custom_key_row   = custom_key_row.clone();
        let custom_model_row = custom_model_row.clone();
        let lang_row         = lang_row.clone();
        let rate_row         = rate_row.clone();
        let history_row         = history_row.clone();
        let vad_row             = vad_row.clone();
        let direct_inject_row   = direct_inject_row.clone();
        let toast_overlay       = toast_overlay.clone();
        let last_saved       = Rc::clone(&last_saved);
        let check_dirty      = Rc::clone(&check_dirty);
        move |_| {
            let lang = lang_row.text().to_string();
            let new_cfg = AppConfig {
                provider:       provider_str(provider_row.selected()).to_string(),
                groq_api_key:   groq_key_row.text().to_string(),
                groq_model:     groq_model_row.text().to_string(),
                cohere_api_key: cohere_key_row.text().to_string(),
                cohere_model:   cohere_model_row.text().to_string(),
                custom_api_url: custom_url_row.text().to_string(),
                custom_api_key: custom_key_row.text().to_string(),
                custom_model:   custom_model_row.text().to_string(),
                language:       if lang.is_empty() { None } else { Some(lang) },
                sample_rate:    rate_row.value() as u32,
                save_history:     history_row.is_active(),
                vad_enabled:      vad_row.is_active(),
                direct_injection: direct_inject_row.is_active(),
            };
            let (title, timeout) = match config::save(&new_cfg) {
                Ok(()) => {
                    *last_saved.borrow_mut() = new_cfg;
                    check_dirty();
                    ("Settings saved", 2)
                }
                Err(e) => {
                    eprintln!("save error: {e}");
                    ("Failed to save — check logs", 4)
                }
            };
            toast_overlay.add_toast(
                adw::Toast::builder().title(title).timeout(timeout).build(),
            );
        }
    });

    page
}

// ── History page ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct HistoryEntry {
    id:        i64,
    timestamp: i64,
    text:      String,
    status:    String,
    error:     Option<String>,
    wav_path:  Option<String>,
}

fn db_path() -> Option<std::path::PathBuf> {
    Some(dirs::data_local_dir()?.join("voice-transcriber").join("history.db"))
}

fn load_history() -> Vec<HistoryEntry> {
    let Some(path) = db_path() else { return vec![] };
    if !path.exists() { return vec![]; }
    let Ok(conn) = Connection::open(&path) else { return vec![]; };

    // Migrate schema if needed (daemon does this too, but app may open first).
    for sql in &[
        "ALTER TABLE history ADD COLUMN status   TEXT NOT NULL DEFAULT 'ok'",
        "ALTER TABLE history ADD COLUMN error    TEXT",
        "ALTER TABLE history ADD COLUMN wav_path TEXT",
    ] {
        let _ = conn.execute(sql, []);
    }

    let Ok(mut stmt) = conn.prepare(
        "SELECT id, timestamp, text, status, error, wav_path
         FROM history ORDER BY id DESC LIMIT 200",
    ) else { return vec![] };

    stmt.query_map([], |row| {
        Ok(HistoryEntry {
            id:        row.get(0)?,
            timestamp: row.get(1)?,
            text:      row.get(2)?,
            status:    row.get(3)?,
            error:     row.get(4)?,
            wav_path:  row.get(5)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

fn delete_history_entry(id: i64, wav_path: Option<&str>) {
    if let Some(path) = wav_path {
        let _ = std::fs::remove_file(path);
    }
    let Some(db) = db_path() else { return };
    if let Ok(conn) = Connection::open(&db) {
        let _ = conn.execute("DELETE FROM history WHERE id = ?1", params![id]);
    }
}

fn clear_history_db() {
    // Collect all WAV paths first, then delete them, then clear the table.
    let Some(db) = db_path() else { return };
    if let Ok(conn) = Connection::open(&db) {
        if let Ok(mut stmt) = conn.prepare("SELECT wav_path FROM history WHERE wav_path IS NOT NULL") {
            if let Ok(paths) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for p in paths.flatten() {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
        let _ = conn.execute("DELETE FROM history", []);
    }
}

fn build_history_page() -> gtk4::Widget {
    let outer = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();

    // ── Toolbar: Clear All ───────────────────────────────────────────────────
    let toolbar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .margin_top(12)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(0)
        .build();

    let clear_btn = gtk4::Button::builder()
        .label("Clear All History")
        .css_classes(vec!["destructive-action"])
        .hexpand(true)
        .build();
    toolbar.append(&clear_btn);
    outer.append(&toolbar);

    // ── Scrolled list ────────────────────────────────────────────────────────
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let list_box = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(vec!["boxed-list"])
        .margin_top(12).margin_bottom(12)
        .margin_start(12).margin_end(12)
        .build();

    scrolled.set_child(Some(&list_box));
    outer.append(&scrolled);

    // ── Refresh closure (shared with row delete/retry buttons) ───────────────
    // We use a two-step Rc so rows built during populate can capture refresh.
    let refresh_holder: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let refresh_fn = {
        let list_box       = list_box.clone();
        let refresh_holder = Rc::clone(&refresh_holder);
        move || populate_history_list(&list_box, &refresh_holder)
    };
    *refresh_holder.borrow_mut() = Some(Box::new(refresh_fn));

    // Initial populate
    populate_history_list(&list_box, &refresh_holder);

    // ── Clear All handler ────────────────────────────────────────────────────
    clear_btn.connect_clicked({
        let refresh_holder = Rc::clone(&refresh_holder);
        move |_| {
            clear_history_db();
            if let Some(f) = refresh_holder.borrow().as_ref() { f(); }
        }
    });

    outer.upcast()
}

fn populate_history_list(
    list_box:       &gtk4::ListBox,
    refresh_holder: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
) {
    // Remove all existing rows.
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let history = load_history();

    if history.is_empty() {
        let empty = adw::ActionRow::builder()
            .title("No transcriptions yet")
            .subtitle("Record something to see your history here")
            .build();
        list_box.append(&empty);
        return;
    }

    for entry in history {
        let row = build_history_row(entry, refresh_holder);
        list_box.append(&row);
    }
}

fn build_history_row(
    entry:          HistoryEntry,
    refresh_holder: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
) -> adw::ActionRow {
    let failed = entry.status == "failed";

    let title = if failed {
        let err = entry.error.as_deref().unwrap_or("unknown error");
        format!("Failed: {}", glib::markup_escape_text(err))
    } else {
        glib::markup_escape_text(&entry.text).into()
    };

    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(format_timestamp(entry.timestamp))
        .build();

    if failed {
        row.add_css_class("error");
    }

    // ── Retry button (failed entries with a saved WAV only) ──────────────────
    if failed {
        if let Some(ref wav_path) = entry.wav_path {
            let spinner = gtk4::Spinner::builder()
                .valign(gtk4::Align::Center)
                .visible(false)
                .build();

            let retry_btn = gtk4::Button::builder()
                .label("Retry")
                .valign(gtk4::Align::Center)
                .css_classes(vec!["suggested-action"])
                .build();

            row.add_suffix(&spinner);
            row.add_suffix(&retry_btn);

            retry_btn.connect_clicked({
                let row            = row.clone();
                let spinner        = spinner.clone();
                let retry_btn      = retry_btn.clone();
                let refresh_holder = Rc::clone(refresh_holder);
                let history_id     = entry.id;
                let wav_path       = wav_path.clone();

                move |btn| {
                    btn.set_visible(false);
                    spinner.set_visible(true);
                    spinner.start();

                    // Call the daemon's RetryTranscription DBus method.
                    let Ok(conn) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
                        spinner.stop();
                        spinner.set_visible(false);
                        retry_btn.set_visible(true);
                        return;
                    };

                    let params = (history_id, wav_path.as_str()).to_variant();
                    let spinner2  = spinner.clone();
                    let retry_btn2 = retry_btn.clone();
                    let refresh_holder2 = Rc::clone(&refresh_holder);
                    let row2 = row.clone();

                    conn.call(
                        Some(common::dbus::SERVICE_NAME),
                        common::dbus::OBJECT_PATH,
                        common::dbus::INTERFACE_NAME,
                        "RetryTranscription",
                        Some(&params),
                        None,
                        gio::DBusCallFlags::NONE,
                        30_000,
                        gio::Cancellable::NONE,
                        move |result| {
                            spinner2.stop();
                            spinner2.set_visible(false);

                            match result {
                                Ok(_) => {
                                    // Success: refresh the whole list so the row updates.
                                    if let Some(f) = refresh_holder2.borrow().as_ref() { f(); }
                                }
                                Err(e) => {
                                    // Failed again — show error on the row subtitle and re-show button.
                                    let msg = e.message();
                                    row2.set_subtitle(&format!("{} — {msg}", format_timestamp(0)));
                                    retry_btn2.set_visible(true);
                                    // Refresh anyway to pick up the updated error stored in DB.
                                    if let Some(f) = refresh_holder2.borrow().as_ref() { f(); }
                                }
                            }
                        },
                    );
                }
            });
        }
    }

    // ── Delete button ────────────────────────────────────────────────────────
    let delete_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["flat"])
        .tooltip_text("Delete this entry")
        .build();

    row.add_suffix(&delete_btn);

    delete_btn.connect_clicked({
        let refresh_holder = Rc::clone(refresh_holder);
        let id             = entry.id;
        let wav_path       = entry.wav_path.clone();
        move |_| {
            delete_history_entry(id, wav_path.as_deref());
            if let Some(f) = refresh_holder.borrow().as_ref() { f(); }
        }
    });

    row
}

fn format_timestamp(ts: i64) -> String {
    if ts == 0 { return String::new(); }
    let elapsed = std::time::SystemTime::now()
        .duration_since(
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64),
        )
        .unwrap_or_default()
        .as_secs();
    match elapsed {
        0..=59       => "just now".into(),
        60..=3599    => format!("{} min ago", elapsed / 60),
        3600..=86399 => format!("{} hr ago", elapsed / 3600),
        s            => format!("{} days ago", s / 86400),
    }
}

// ── Status page ───────────────────────────────────────────────────────────────

enum FixAction {
    RunCommand {
        btn_label:   &'static str,
        cmd_display: &'static str,
        program:     &'static str,
        args:        &'static [&'static str],
    },
    CopyText {
        btn_label: &'static str,
        text:      &'static str,
    },
    GoToSettings,
}

struct StatusCheck {
    title:    &'static str,
    ok_msg:   &'static str,
    fail_msg: &'static str,
    run:      fn() -> bool,
    fix:      Option<FixAction>,
}

fn check_daemon_installed() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".local/bin/voice-transcriber-daemon").exists())
        .unwrap_or(false)
}

fn check_daemon_running() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "voice-transcriber-daemon.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn check_extension_installed() -> bool {
    dirs::data_local_dir()
        .map(|d| d.join("gnome-shell/extensions/voice-transcriber@local").exists())
        .unwrap_or(false)
}

fn check_extension_loaded() -> bool {
    std::process::Command::new("gnome-extensions")
        .args(["list"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("voice-transcriber@local"))
        .unwrap_or(false)
}

fn check_extension_enabled() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.shell", "enabled-extensions"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("voice-transcriber@local"))
        .unwrap_or(false)
}

fn check_api_key() -> bool {
    config::load().map(|c| !c.active_key().is_empty()).unwrap_or(false)
}

const CHECKS: &[StatusCheck] = &[
    StatusCheck {
        title:    "Daemon installed",
        ok_msg:   "Binary found at ~/.local/bin/voice-transcriber-daemon",
        fail_msg: "Binary not found — run install.sh from the repository",
        run:  check_daemon_installed,
        fix:  Some(FixAction::CopyText {
            btn_label: "Copy command",
            text:      "bash install.sh",
        }),
    },
    StatusCheck {
        title:    "Daemon running",
        ok_msg:   "systemd service is active",
        fail_msg: "Service is not running — click Start to enable it",
        run:  check_daemon_running,
        fix:  Some(FixAction::RunCommand {
            btn_label:   "Start daemon",
            cmd_display: "systemctl --user enable --now voice-transcriber-daemon.service",
            program:     "systemctl",
            args:        &["--user", "enable", "--now", "voice-transcriber-daemon.service"],
        }),
    },
    StatusCheck {
        title:    "Extension installed",
        ok_msg:   "Found in ~/.local/share/gnome-shell/extensions/",
        fail_msg: "Extension not found — run install.sh from the repository",
        run:  check_extension_installed,
        fix:  Some(FixAction::CopyText {
            btn_label: "Copy command",
            text:      "bash install.sh",
        }),
    },
    StatusCheck {
        title:    "Extension loaded by GNOME Shell",
        ok_msg:   "GNOME Shell has scanned the extension",
        fail_msg: "GNOME Shell hasn't loaded the extension yet. On Wayland, log out and back in. On X11, press Alt+F2 → r.",
        run:  check_extension_loaded,
        fix:  Some(FixAction::CopyText {
            btn_label: "Copy logout command",
            text: "gnome-session-quit --logout",
        }),
    },
    StatusCheck {
        title:    "Extension enabled",
        ok_msg:   "Enabled in GNOME Shell",
        fail_msg: "Extension is loaded but not yet enabled",
        run:  check_extension_enabled,
        fix:  Some(FixAction::RunCommand {
            btn_label:   "Enable extension",
            cmd_display: "gnome-extensions enable voice-transcriber@local",
            program:     "gnome-extensions",
            args:        &["enable", "voice-transcriber@local"],
        }),
    },
    StatusCheck {
        title:    "API key configured",
        ok_msg:   "API key is set",
        fail_msg: "No API key — transcription will fail without one",
        run:  check_api_key,
        fix:  Some(FixAction::GoToSettings),
    },
];

fn apply_expander_state(
    expander: &adw::ExpanderRow,
    icon:     &gtk4::Image,
    btn:      Option<&gtk4::Button>,
    check:    &StatusCheck,
    ok:       bool,
) {
    if ok {
        expander.set_subtitle(check.ok_msg);
        expander.set_enable_expansion(false);
        expander.set_expanded(false);
        icon.set_icon_name(Some("object-select-symbolic"));
        icon.remove_css_class("warning");
        icon.add_css_class("success");
        if let Some(b) = btn { b.set_visible(false); }
    } else {
        expander.set_subtitle(check.fail_msg);
        expander.set_enable_expansion(true);
        icon.set_icon_name(Some("dialog-warning-symbolic"));
        icon.remove_css_class("success");
        icon.add_css_class("warning");
        if let Some(b) = btn { b.set_visible(true); }
    }
}

fn apply_action_state(
    row:   &adw::ActionRow,
    icon:  &gtk4::Image,
    btn:   Option<&gtk4::Button>,
    check: &StatusCheck,
    ok:    bool,
) {
    if ok {
        row.set_subtitle(check.ok_msg);
        icon.set_icon_name(Some("object-select-symbolic"));
        icon.remove_css_class("warning");
        icon.add_css_class("success");
        if let Some(b) = btn { b.set_visible(false); }
    } else {
        row.set_subtitle(check.fail_msg);
        icon.set_icon_name(Some("dialog-warning-symbolic"));
        icon.remove_css_class("success");
        icon.add_css_class("warning");
        if let Some(b) = btn { b.set_visible(true); }
    }
}

fn build_run_row(
    check:       &'static StatusCheck,
    btn_label:   &'static str,
    cmd_display: &'static str,
    program:     &'static str,
    args:        &'static [&'static str],
) -> (adw::ExpanderRow, Box<dyn Fn()>) {
    let expander = adw::ExpanderRow::builder()
        .title(check.title)
        .show_enable_switch(false)
        .build();

    let icon = gtk4::Image::new();
    icon.set_pixel_size(16);
    icon.set_valign(gtk4::Align::Center);

    let spinner = gtk4::Spinner::builder()
        .valign(gtk4::Align::Center)
        .visible(false)
        .build();

    let run_btn = gtk4::Button::builder()
        .label(btn_label)
        .valign(gtk4::Align::Center)
        .css_classes(vec!["suggested-action"])
        .visible(false)
        .build();

    expander.add_suffix(&icon);
    expander.add_suffix(&run_btn);
    expander.add_suffix(&spinner);

    let cmd_label = gtk4::Label::builder()
        .label(format!("$ {cmd_display}"))
        .halign(gtk4::Align::Start)
        .selectable(true)
        .css_classes(vec!["monospace"])
        .margin_top(10)
        .margin_bottom(6)
        .margin_start(18)
        .margin_end(12)
        .build();

    let output_label = gtk4::Label::builder()
        .halign(gtk4::Align::Start)
        .selectable(true)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .css_classes(vec!["monospace", "caption", "dim-label"])
        .margin_bottom(10)
        .margin_start(18)
        .margin_end(12)
        .visible(false)
        .build();

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content.append(&cmd_label);
    content.append(&output_label);
    expander.add_row(&content);

    {
        let run_btn   = run_btn.clone();
        let spinner   = spinner.clone();
        let output    = output_label.clone();
        let expander  = expander.clone();
        let icon      = icon.clone();

        run_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            btn.set_visible(false);
            spinner.set_visible(true);
            spinner.start();
            output.set_text("Running…");
            output.set_visible(true);
            expander.set_expanded(true);

            let (tx, rx) = std::sync::mpsc::sync_channel::<std::io::Result<std::process::Output>>(1);
            std::thread::spawn(move || {
                let _ = tx.send(std::process::Command::new(program).args(args).output());
            });

            let btn2      = btn.clone();
            let spinner2  = spinner.clone();
            let output2   = output.clone();
            let expander2 = expander.clone();
            let icon2     = icon.clone();

            glib::idle_add_local(move || {
                let res = match rx.try_recv() {
                    Ok(r)  => r,
                    Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                    Err(_) => return glib::ControlFlow::Break,
                };
                spinner2.stop();
                spinner2.set_visible(false);

                match res {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let combined = format!("{stdout}{stderr}").trim().to_string();

                        if out.status.success() {
                            let msg = if combined.is_empty() {
                                "Completed successfully.".to_string()
                            } else {
                                combined
                            };
                            output2.set_text(&msg);
                            let ok = (check.run)();
                            apply_expander_state(&expander2, &icon2, Some(&btn2), check, ok);
                            if !ok {
                                btn2.set_sensitive(true);
                            }
                        } else {
                            let code = out.status.code().unwrap_or(-1);
                            let msg = if combined.is_empty() {
                                format!("Command failed (exit {code})")
                            } else {
                                format!("Exit {code}:\n{combined}")
                            };
                            output2.set_text(&msg);
                            btn2.set_sensitive(true);
                            btn2.set_visible(true);
                        }
                    }
                    Err(e) => {
                        output2.set_text(&format!("Could not run command: {e}"));
                        output2.set_visible(true);
                        btn2.set_sensitive(true);
                        btn2.set_visible(true);
                    }
                }
                glib::ControlFlow::Break
            });
        });
    }

    apply_expander_state(&expander, &icon, Some(&run_btn), check, (check.run)());

    let refresh: Box<dyn Fn()> = {
        let expander = expander.clone();
        let icon     = icon.clone();
        let run_btn  = run_btn.clone();
        Box::new(move || {
            apply_expander_state(&expander, &icon, Some(&run_btn), check, (check.run)())
        })
    };

    (expander, refresh)
}

fn build_copy_row(
    check:     &'static StatusCheck,
    btn_label: &'static str,
    text:      &'static str,
) -> (adw::ExpanderRow, Box<dyn Fn()>) {
    let expander = adw::ExpanderRow::builder()
        .title(check.title)
        .show_enable_switch(false)
        .build();

    let icon = gtk4::Image::new();
    icon.set_pixel_size(16);
    icon.set_valign(gtk4::Align::Center);

    let copy_btn = gtk4::Button::builder()
        .label(btn_label)
        .valign(gtk4::Align::Center)
        .css_classes(vec!["suggested-action"])
        .visible(false)
        .build();

    expander.add_suffix(&icon);
    expander.add_suffix(&copy_btn);

    let cmd_label = gtk4::Label::builder()
        .label(format!("$ {text}"))
        .halign(gtk4::Align::Start)
        .selectable(true)
        .css_classes(vec!["monospace"])
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(18)
        .margin_end(12)
        .build();
    expander.add_row(&cmd_label);

    {
        let copy_btn = copy_btn.clone();
        copy_btn.connect_clicked(move |btn| {
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(text);
            }
            btn.set_label("Copied!");
            btn.remove_css_class("suggested-action");
            btn.add_css_class("success");
            let btn2 = btn.clone();
            glib::timeout_add_local_once(Duration::from_secs(2), move || {
                btn2.set_label(btn_label);
                btn2.remove_css_class("success");
                btn2.add_css_class("suggested-action");
            });
        });
    }

    apply_expander_state(&expander, &icon, Some(&copy_btn), check, (check.run)());

    let refresh: Box<dyn Fn()> = {
        let expander = expander.clone();
        let icon     = icon.clone();
        let copy_btn = copy_btn.clone();
        Box::new(move || {
            apply_expander_state(&expander, &icon, Some(&copy_btn), check, (check.run)())
        })
    };

    (expander, refresh)
}

fn build_goto_row(
    check:      &'static StatusCheck,
    view_stack: &adw::ViewStack,
) -> (adw::ActionRow, Box<dyn Fn()>) {
    let icon = gtk4::Image::new();
    icon.set_pixel_size(16);
    icon.set_valign(gtk4::Align::Center);

    let goto_btn = gtk4::Button::builder()
        .label("Go to Settings")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["suggested-action"])
        .visible(false)
        .build();

    let row = adw::ActionRow::builder()
        .title(check.title)
        .build();
    row.add_suffix(&icon);
    row.add_suffix(&goto_btn);

    let vs = view_stack.clone();
    goto_btn.connect_clicked(move |_| {
        vs.set_visible_child_name("settings");
    });

    apply_action_state(&row, &icon, Some(&goto_btn), check, (check.run)());

    let refresh: Box<dyn Fn()> = {
        let row      = row.clone();
        let icon     = icon.clone();
        let goto_btn = goto_btn.clone();
        Box::new(move || {
            apply_action_state(&row, &icon, Some(&goto_btn), check, (check.run)())
        })
    };

    (row, refresh)
}

fn build_status_page(view_stack: &adw::ViewStack) -> adw::PreferencesPage {
    let page  = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .title("Installation & Configuration")
        .build();

    let refresh_fns: Rc<RefCell<Vec<Box<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

    for check in CHECKS {
        match &check.fix {
            Some(FixAction::RunCommand { btn_label, cmd_display, program, args }) => {
                let (row, f) = build_run_row(check, btn_label, cmd_display, *program, *args);
                group.add(&row);
                refresh_fns.borrow_mut().push(f);
            }
            Some(FixAction::CopyText { btn_label, text }) => {
                let (row, f) = build_copy_row(check, btn_label, text);
                group.add(&row);
                refresh_fns.borrow_mut().push(f);
            }
            Some(FixAction::GoToSettings) => {
                let (row, f) = build_goto_row(check, view_stack);
                group.add(&row);
                refresh_fns.borrow_mut().push(f);
            }
            None => {
                let expander = adw::ExpanderRow::builder()
                    .title(check.title)
                    .show_enable_switch(false)
                    .build();
                let icon = gtk4::Image::new();
                icon.set_pixel_size(16);
                icon.set_valign(gtk4::Align::Center);
                expander.add_suffix(&icon);
                apply_expander_state(&expander, &icon, None, check, (check.run)());
                let f: Box<dyn Fn()> = {
                    let expander = expander.clone();
                    let icon     = icon.clone();
                    Box::new(move || {
                        apply_expander_state(&expander, &icon, None, check, (check.run)())
                    })
                };
                group.add(&expander);
                refresh_fns.borrow_mut().push(f);
            }
        }
    }

    let refresh_group = adw::PreferencesGroup::new();
    let recheck_btn = adw::ButtonRow::builder()
        .title("Recheck All")
        .start_icon_name("view-refresh-symbolic")
        .build();
    recheck_btn.connect_activated(move |_| {
        for f in refresh_fns.borrow().iter() { f(); }
    });
    refresh_group.add(&recheck_btn);

    page.add(&group);
    page.add(&refresh_group);
    page
}
