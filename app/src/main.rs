use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use common::config::{self, AppConfig};
use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;
use rusqlite::{params, Connection};

// ── Auto-save ────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AutoSaver {
    config: Rc<RefCell<AppConfig>>,
    toast:  adw::ToastOverlay,
    timer:  Rc<RefCell<Option<glib::SourceId>>>,
}

impl AutoSaver {
    fn new(cfg: AppConfig, toast: &adw::ToastOverlay) -> Self {
        Self {
            config: Rc::new(RefCell::new(cfg)),
            toast:  toast.clone(),
            timer:  Rc::new(RefCell::new(None)),
        }
    }

    fn save(&self) {
        if let Some(id) = self.timer.borrow_mut().take() {
            id.remove();
        }
        let cfg   = Rc::clone(&self.config);
        let toast = self.toast.clone();
        let timer = Rc::clone(&self.timer);
        *self.timer.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(300),
            move || {
                *timer.borrow_mut() = None;
                if let Err(e) = config::save(&cfg.borrow()) {
                    eprintln!("save error: {e}");
                    toast.add_toast(
                        adw::Toast::builder()
                            .title("Failed to save settings")
                            .timeout(4)
                            .build(),
                    );
                    return;
                }
                notify_daemon_reload();
            },
        ));
    }

    fn update(&self, f: impl FnOnce(&mut AppConfig)) {
        f(&mut self.config.borrow_mut());
        self.save();
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Tell the running daemon to reload its in-memory config from disk.
/// Silently no-ops if the daemon isn't running or the call fails — the next
/// time the daemon starts it will load the on-disk config anyway.
fn notify_daemon_reload() {
    let Ok(conn) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return;
    };
    conn.call(
        Some(common::dbus::SERVICE_NAME),
        common::dbus::OBJECT_PATH,
        common::dbus::INTERFACE_NAME,
        "ReloadConfig",
        None,
        None,
        gio::DBusCallFlags::NONE,
        5_000,
        gio::Cancellable::NONE,
        |_| {},
    );
}

fn provider_idx(p: &str) -> u32 {
    match p { "cohere" => 1, "custom" => 2, _ => 0 }
}

fn provider_str(idx: u32) -> &'static str {
    match idx { 1 => "cohere", 2 => "custom", _ => "groq" }
}

fn load_ext_settings() -> Option<gio::Settings> {
    let schema_dir = dirs::home_dir()?
        .join(".local/share/gnome-shell/extensions/transcriber@local/schemas");
    let source = gio::SettingsSchemaSource::from_directory(
        schema_dir,
        gio::SettingsSchemaSource::default().as_ref(),
        false,
    )
    .ok()?;
    let schema = source.lookup(
        "org.gnome.shell.extensions.transcriber",
        false,
    )?;
    Some(gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None))
}

fn build_shortcuts_dialog(ext: &Option<gio::Settings>) -> adw::ShortcutsDialog {
    let read_accel = |key: &str, fallback: &str| -> String {
        ext.as_ref()
            .map(|s| s.strv(key))
            .and_then(|v| v.first().map(|g| g.to_string()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback.to_string())
    };

    let dialog = adw::ShortcutsDialog::new();

    let recording = adw::ShortcutsSection::new(Some("Recording"));
    recording.add(adw::ShortcutsItem::new(
        "Toggle Recording",
        &read_accel("toggle-recording", "<Super>apostrophe"),
    ));
    recording.add(adw::ShortcutsItem::new(
        "Cancel Recording",
        &read_accel("cancel-recording", "Escape"),
    ));
    dialog.add(recording);

    let app_section = adw::ShortcutsSection::new(Some("Application"));
    app_section.add(adw::ShortcutsItem::new("Settings",        "<Control>1"));
    app_section.add(adw::ShortcutsItem::new("Post-Processing", "<Control>2"));
    app_section.add(adw::ShortcutsItem::new("History",         "<Control>3"));
    app_section.add(adw::ShortcutsItem::new("Status",          "<Control>4"));
    app_section.add(adw::ShortcutsItem::new(
        "Keyboard Shortcuts",
        "<Control>question",
    ));
    app_section.add(adw::ShortcutsItem::new("Quit", "<Control>q"));
    dialog.add(app_section);

    dialog
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    adw::init().expect("failed to initialise libadwaita");
    let app = adw::Application::builder()
        .application_id("org.transcriber.Settings")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    let toast_overlay = adw::ToastOverlay::new();
    let cfg = config::load().unwrap_or_default();
    let saver = AutoSaver::new(cfg, &toast_overlay);
    let ext_settings = load_ext_settings();

    // ── Pages ───────────────────────────────────────────────────────────────
    let view_stack = adw::ViewStack::new();

    let (settings_widget, streaming_expander) = build_settings_page(&saver, &ext_settings);
    view_stack.add_titled_with_icon(
        &settings_widget,
        Some("settings"),
        "Settings",
        "preferences-system-symbolic",
    );

    let (postprocess_widget, postprocess_switch) = build_postprocess_page(&saver);
    view_stack.add_titled_with_icon(
        &postprocess_widget,
        Some("postprocess"),
        "Post-Processing",
        "applications-utilities-symbolic",
    );

    // Mutual exclusion: streaming and post-processing can't both be on. When
    // the user flips one on, force the other off and surface a toast.
    streaming_expander.connect_enable_expansion_notify({
        let saver = saver.clone();
        let postprocess_switch = postprocess_switch.clone();
        let toast_overlay = toast_overlay.clone();
        move |row| {
            if row.enables_expansion() && saver.config.borrow().postprocess_enabled {
                postprocess_switch.set_active(false);
                saver.update(|cfg| cfg.postprocess_enabled = false);
                toast_overlay.add_toast(
                    adw::Toast::builder()
                        .title("Post-processing disabled — incompatible with streaming")
                        .timeout(4)
                        .build(),
                );
            }
        }
    });
    postprocess_switch.connect_active_notify({
        let saver = saver.clone();
        let streaming_expander = streaming_expander.clone();
        let toast_overlay = toast_overlay.clone();
        move |row| {
            let enabled = row.is_active();
            saver.update(|cfg| cfg.postprocess_enabled = enabled);
            if enabled && saver.config.borrow().streaming_enabled {
                streaming_expander.set_enable_expansion(false);
                saver.update(|cfg| cfg.streaming_enabled = false);
                toast_overlay.add_toast(
                    adw::Toast::builder()
                        .title("Streaming disabled — incompatible with post-processing")
                        .timeout(4)
                        .build(),
                );
            }
        }
    });

    let (history_widget, history_refresh) = build_history_page(&toast_overlay);
    view_stack.add_titled_with_icon(
        &history_widget,
        Some("history"),
        "History",
        "document-open-recent-symbolic",
    );

    let (status_page, status_refresh) = build_status_page(&toast_overlay, &view_stack);
    view_stack.add_titled_with_icon(
        &status_page,
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

    let clear_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Clear All History")
        .visible(false)
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&switcher));
    header.pack_end(&menu_btn);
    header.pack_end(&clear_btn);

    // ── Window ──────────────────────────────────────────────────────────────
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
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

    // ── Actions ─────────────────────────────────────────────────────────────
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate({
        let w = window.clone();
        move |_, _| {
            adw::AboutDialog::builder()
                .application_name("Transcriber")
                .application_icon("audio-input-microphone-symbolic")
                .developer_name("Craig")
                .version("0.1.0")
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
        app.set_accels_for_action(
            &format!("win.go-{name}"),
            &[&format!("<Control>{}", i + 1)],
        );
    }

    // ── Page visibility logic ───────────────────────────────────────────────
    view_stack.connect_visible_child_name_notify({
        let clear_btn = clear_btn.clone();
        let status_refresh = Rc::clone(&status_refresh);
        let history_refresh_tab = Rc::clone(&history_refresh);
        move |stack| {
            let name = stack.visible_child_name();
            let name = name.as_deref();
            clear_btn.set_visible(name == Some("history"));
            if name == Some("history") {
                history_refresh_tab();
            }
            if name == Some("status") {
                status_refresh();
            }
        }
    });

    clear_btn.connect_clicked({
        let w = window.clone();
        let refresh = Rc::clone(&history_refresh);
        move |_| {
            let dialog = adw::AlertDialog::builder()
                .heading("Clear All History?")
                .body("This will permanently delete all transcriptions and saved recordings.")
                .build();
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("clear", "Clear All");
            dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            dialog.connect_response(None, {
                let refresh = Rc::clone(&refresh);
                move |_, resp| {
                    if resp == "clear" {
                        clear_history_db();
                        refresh();
                    }
                }
            });
            dialog.present(Some(&w));
        }
    });

    // Auto-refresh history by polling the database for new entries
    {
        let refresh = Rc::clone(&history_refresh);
        let last_max_id: Rc<RefCell<i64>> = Rc::new(RefCell::new(latest_history_id()));
        glib::timeout_add_seconds_local(2, move || {
            let current = latest_history_id();
            let prev = *last_max_id.borrow();
            if current != prev {
                *last_max_id.borrow_mut() = current;
                refresh();
            }
            glib::ControlFlow::Continue
        });
    }

    window.present();
}

// ── Settings page ────────────────────────────────────────────────────────────

fn build_settings_page(
    saver: &AutoSaver,
    ext_settings: &Option<gio::Settings>,
) -> (adw::PreferencesPage, adw::ExpanderRow) {
    let page = adw::PreferencesPage::new();

    // ── Provider group ──────────────────────────────────────────────────────
    let provider_group = adw::PreferencesGroup::builder()
        .title("Transcription Provider")
        .build();

    let provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Groq", "Cohere", "Custom"]))
        .build();

    let cfg = saver.config.borrow();
    provider_row.set_selected(provider_idx(&cfg.provider));

    let api_key_row = adw::PasswordEntryRow::builder()
        .title("API Key")
        .text(cfg.active_key())
        .build();

    let model_row = adw::EntryRow::builder()
        .title("Model")
        .text(cfg.active_model())
        .build();

    let url_row = adw::EntryRow::builder()
        .title("API URL")
        .text(&cfg.custom_api_url)
        .visible(cfg.provider == "custom")
        .build();

    let lang_row = adw::EntryRow::builder()
        .title("Language (BCP-47)")
        .text(cfg.language.as_deref().unwrap_or(""))
        .build();

    drop(cfg);

    provider_row.connect_selected_notify({
        let saver = saver.clone();
        let api_key_row = api_key_row.clone();
        let model_row = model_row.clone();
        let url_row = url_row.clone();
        move |row| {
            {
                let mut cfg = saver.config.borrow_mut();
                let key   = api_key_row.text().to_string();
                let model = model_row.text().to_string();
                let url   = url_row.text().to_string();
                match cfg.provider.as_str() {
                    "groq" => {
                        cfg.groq_api_key = key;
                        cfg.groq_model   = model;
                    }
                    "cohere" => {
                        cfg.cohere_api_key = key;
                        cfg.cohere_model   = model;
                    }
                    _ => {
                        cfg.custom_api_key = key;
                        cfg.custom_model   = model;
                        cfg.custom_api_url = url;
                    }
                }
                cfg.provider = provider_str(row.selected()).to_string();
            }
            let (new_key, new_model, new_url) = {
                let c = saver.config.borrow();
                (
                    c.active_key().to_string(),
                    c.active_model().to_string(),
                    c.custom_api_url.clone(),
                )
            };
            api_key_row.set_text(&new_key);
            model_row.set_text(&new_model);
            url_row.set_text(&new_url);
            url_row.set_visible(provider_str(row.selected()) == "custom");
            saver.save();
        }
    });

    api_key_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(|cfg| match cfg.provider.as_str() {
                "groq"   => cfg.groq_api_key   = t,
                "cohere" => cfg.cohere_api_key  = t,
                _        => cfg.custom_api_key  = t,
            });
        }
    });

    model_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(|cfg| match cfg.provider.as_str() {
                "groq"   => cfg.groq_model   = t,
                "cohere" => cfg.cohere_model  = t,
                _        => cfg.custom_model  = t,
            });
        }
    });

    url_row.connect_changed({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.custom_api_url = row.text().to_string())
    });

    lang_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(|cfg| cfg.language = if t.is_empty() { None } else { Some(t) });
        }
    });

    provider_group.add(&provider_row);
    provider_group.add(&api_key_row);
    provider_group.add(&model_row);
    provider_group.add(&url_row);
    provider_group.add(&lang_row);

    // ── Vocabulary group ────────────────────────────────────────────────────
    let vocab_group = adw::PreferencesGroup::builder()
        .title("Vocabulary")
        .description(
            "Spelling hints sent with every request — one term per line. \
             Whisper reads them as the text immediately preceding your audio, \
             which biases it toward these spellings for product names and jargon.",
        )
        .build();

    let vocab_enable_row = adw::SwitchRow::builder()
        .title("Send vocabulary")
        .subtitle("Omits the hint entirely when off")
        .active(saver.config.borrow().vocabulary_enabled)
        .build();
    vocab_enable_row.connect_active_notify({
        let saver = saver.clone();
        move |row| {
            let on = row.is_active();
            saver.update(|cfg| cfg.vocabulary_enabled = on);
        }
    });

    let vocab_buffer = gtk4::TextBuffer::new(None);
    vocab_buffer.set_text(&saver.config.borrow().vocabulary.join("\n"));

    let vocab_view = gtk4::TextView::builder()
        .buffer(&vocab_buffer)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();

    let vocab_scroll = gtk4::ScrolledWindow::builder()
        .height_request(140)
        .child(&vocab_view)
        .build();

    let vocab_frame = gtk4::Frame::builder().child(&vocab_scroll).build();
    vocab_frame.add_css_class("card");

    // Live budget readout. Whisper drops everything past 224 tokens without
    // reporting it, so the only way a user learns their last few terms stopped
    // working is if the GUI says so before the request is ever sent.
    let vocab_counter = gtk4::Label::builder().halign(gtk4::Align::Start).build();
    vocab_counter.add_css_class("caption");
    vocab_counter.add_css_class("dim-label");

    let update_vocab_counter = {
        let vocab_counter = vocab_counter.clone();
        move |terms: &[String]| {
            let tokens = config::estimate_prompt_tokens(terms);
            let limit = config::VOCABULARY_TOKEN_LIMIT;
            vocab_counter.set_label(&format!(
                "{} term{} · ~{tokens}/{limit} tokens",
                terms.len(),
                if terms.len() == 1 { "" } else { "s" },
            ));
            if tokens > limit {
                vocab_counter.remove_css_class("dim-label");
                vocab_counter.add_css_class("error");
                vocab_counter.set_tooltip_text(Some(
                    "Over Whisper's 224-token cap — terms past the limit are \
                     dropped before sending. Estimate is deliberately high.",
                ));
            } else {
                vocab_counter.remove_css_class("error");
                vocab_counter.add_css_class("dim-label");
                vocab_counter.set_tooltip_text(None);
            }
        }
    };
    update_vocab_counter(&saver.config.borrow().vocabulary);

    vocab_buffer.connect_changed({
        let saver = saver.clone();
        move |buf| {
            let text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();
            let terms: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            update_vocab_counter(&terms);
            saver.update(|cfg| cfg.vocabulary = terms);
        }
    });

    let vocab_reset = gtk4::Button::builder()
        .label("Reset to Default")
        .halign(gtk4::Align::End)
        .build();
    vocab_reset.connect_clicked({
        let vocab_buffer = vocab_buffer.clone();
        move |_| vocab_buffer.set_text(&config::DEFAULT_VOCABULARY.join("\n"))
    });

    let vocab_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    vocab_box.append(&vocab_frame);
    vocab_box.append(&vocab_counter);
    vocab_box.append(&vocab_reset);

    vocab_group.add(&vocab_enable_row);
    vocab_group.add(&vocab_box);

    // Cohere's endpoint documents no `prompt` field, and `active_prompt()`
    // withholds the hint there rather than risk a 400 on every request. Grey the
    // group out to match, instead of leaving controls that silently do nothing.
    // Registered as a *second* handler on the same signal — the one above owns
    // migrating the key/model/URL fields, and this one only reads `selected()`,
    // so the two are order-independent.
    let sync_vocab_sensitivity = {
        let vocab_group = vocab_group.clone();
        move |provider: &str| {
            let supported = provider != "cohere";
            vocab_group.set_sensitive(supported);
            vocab_group.set_tooltip_text(if supported {
                None
            } else {
                Some("Cohere's transcription endpoint accepts no vocabulary hint.")
            });
        }
    };
    sync_vocab_sensitivity(&saver.config.borrow().provider);
    provider_row
        .connect_selected_notify(move |row| sync_vocab_sensitivity(provider_str(row.selected())));

    // ── Behaviour group ─────────────────────────────────────────────────────
    let behaviour_group = adw::PreferencesGroup::builder()
        .title("Behaviour")
        .build();

    let paste_row = adw::SwitchRow::builder()
        .title("Auto-paste after transcription")
        .subtitle("Pastes into the focused window")
        .build();
    if let Some(ref s) = ext_settings {
        paste_row.set_active(s.boolean("auto-paste"));
        paste_row.connect_active_notify({
            let s = s.clone();
            move |row| {
                let _ = s.set_boolean("auto-paste", row.is_active());
            }
        });
    } else {
        paste_row.set_sensitive(false);
        paste_row.set_subtitle("Extension not installed");
    }

    let vad_row = adw::SwitchRow::builder()
        .title("Auto-stop on silence")
        .subtitle("Stop recording after ~1.5 s of silence")
        .active(saver.config.borrow().vad_enabled)
        .build();
    vad_row.connect_active_notify({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.vad_enabled = row.is_active())
    });

    let direct_inject_row = adw::SwitchRow::builder()
        .title("Direct text injection")
        .subtitle("Type text directly instead of using clipboard")
        .active(saver.config.borrow().direct_injection)
        .build();
    if let Some(ref s) = ext_settings {
        if s.boolean("direct-injection") {
            direct_inject_row.set_active(true);
        }
        direct_inject_row.connect_active_notify({
            let saver = saver.clone();
            let s = s.clone();
            move |row| {
                let v = row.is_active();
                let _ = s.set_boolean("direct-injection", v);
                saver.update(|cfg| cfg.direct_injection = v);
            }
        });
    } else {
        direct_inject_row.connect_active_notify({
            let saver = saver.clone();
            move |row| saver.update(|cfg| cfg.direct_injection = row.is_active())
        });
    }

    let audio_cues_row = adw::SwitchRow::builder()
        .title("Audio feedback cues")
        .subtitle("Sounds on recording start and transcription finish")
        .active(saver.config.borrow().audio_cues_enabled)
        .build();
    audio_cues_row.connect_active_notify({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.audio_cues_enabled = row.is_active())
    });

    let history_row = adw::SwitchRow::builder()
        .title("Save transcription history")
        .subtitle("Record WAV files and results to disk")
        .active(saver.config.borrow().save_history)
        .build();
    history_row.connect_active_notify({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.save_history = row.is_active())
    });

    behaviour_group.add(&paste_row);
    behaviour_group.add(&vad_row);
    behaviour_group.add(&direct_inject_row);
    behaviour_group.add(&audio_cues_row);
    behaviour_group.add(&history_row);

    // ── Streaming group ─────────────────────────────────────────────────────
    let streaming_group = adw::PreferencesGroup::new();

    let cfg = saver.config.borrow();

    let streaming_expander = adw::ExpanderRow::builder()
        .title("Real-Time Streaming")
        .subtitle("WebSocket-based transcription with ~300 ms latency")
        .show_enable_switch(true)
        .enable_expansion(cfg.streaming_enabled)
        .build();

    fn streaming_prov_idx(p: &str) -> u32 {
        if p == "assemblyai" { 1 } else { 0 }
    }

    let stream_provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Deepgram", "AssemblyAI"]))
        .build();
    stream_provider_row.set_selected(streaming_prov_idx(&cfg.streaming_provider));

    let dg_key_row = adw::PasswordEntryRow::builder()
        .title("Deepgram API Key")
        .text(&cfg.deepgram_api_key)
        .visible(streaming_prov_idx(&cfg.streaming_provider) == 0)
        .build();

    let dg_model_row = adw::EntryRow::builder()
        .title("Deepgram Model")
        .text(&cfg.deepgram_model)
        .visible(streaming_prov_idx(&cfg.streaming_provider) == 0)
        .build();

    let aai_key_row = adw::PasswordEntryRow::builder()
        .title("AssemblyAI API Key")
        .text(&cfg.assemblyai_api_key)
        .visible(streaming_prov_idx(&cfg.streaming_provider) == 1)
        .build();

    drop(cfg);

    streaming_expander.connect_enable_expansion_notify({
        let saver = saver.clone();
        let direct_inject_row = direct_inject_row.clone();
        let ext_settings = ext_settings.clone();
        move |row| {
            let enabled = row.enables_expansion();
            if enabled {
                row.set_expanded(true);
                direct_inject_row.set_active(true);
                if let Some(ref s) = ext_settings {
                    let _ = s.set_boolean("direct-injection", true);
                }
            }
            saver.update(|cfg| cfg.streaming_enabled = enabled);
        }
    });

    stream_provider_row.connect_selected_notify({
        let dg_key   = dg_key_row.clone();
        let dg_model = dg_model_row.clone();
        let aai_key  = aai_key_row.clone();
        let saver    = saver.clone();
        move |row| {
            let is_dg = row.selected() == 0;
            dg_key.set_visible(is_dg);
            dg_model.set_visible(is_dg);
            aai_key.set_visible(!is_dg);
            saver.update(|cfg| {
                cfg.streaming_provider =
                    if is_dg { "deepgram" } else { "assemblyai" }.to_string();
            });
        }
    });

    dg_key_row.connect_changed({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.deepgram_api_key = row.text().to_string())
    });
    dg_model_row.connect_changed({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.deepgram_model = row.text().to_string())
    });
    aai_key_row.connect_changed({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.assemblyai_api_key = row.text().to_string())
    });

    streaming_expander.add_row(&stream_provider_row);
    streaming_expander.add_row(&dg_key_row);
    streaming_expander.add_row(&dg_model_row);
    streaming_expander.add_row(&aai_key_row);
    streaming_group.add(&streaming_expander);

    // ── Audio group ─────────────────────────────────────────────────────────
    let audio_group = adw::PreferencesGroup::builder().title("Audio").build();

    let adj = gtk4::Adjustment::builder()
        .lower(8000.0)
        .upper(48000.0)
        .step_increment(1000.0)
        .page_increment(8000.0)
        .value(saver.config.borrow().sample_rate as f64)
        .build();
    let rate_row = adw::SpinRow::builder()
        .title("Sample Rate (Hz)")
        .subtitle("16 000 Hz recommended for Whisper")
        .adjustment(&adj)
        .build();
    rate_row.connect_value_notify({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.sample_rate = row.value() as u32)
    });
    audio_group.add(&rate_row);

    // ── Assemble ────────────────────────────────────────────────────────────
    page.add(&provider_group);
    page.add(&vocab_group);
    page.add(&behaviour_group);
    page.add(&streaming_group);
    page.add(&audio_group);
    (page, streaming_expander)
}

// ── Post-processing page ─────────────────────────────────────────────────────

fn postprocess_provider_idx(p: &str) -> u32 {
    match p { "gemini" => 1, "custom" => 2, _ => 0 }
}

fn postprocess_provider_str(idx: u32) -> &'static str {
    match idx { 1 => "gemini", 2 => "custom", _ => "groq" }
}

fn build_postprocess_page(saver: &AutoSaver) -> (adw::PreferencesPage, adw::SwitchRow) {
    let page = adw::PreferencesPage::new();

    // ── Enable group ────────────────────────────────────────────────────────
    let enable_group = adw::PreferencesGroup::builder()
        .title("Post-Processing")
        .description(
            "Run the raw transcription through a chat LLM to fix recognition errors \
             and punctuation before delivering. Mutually exclusive with streaming.",
        )
        .build();

    let enable_row = adw::SwitchRow::builder()
        .title("Enable post-processing")
        .subtitle("Disables streaming when turned on")
        .active(saver.config.borrow().postprocess_enabled)
        .build();
    enable_group.add(&enable_row);

    // ── Provider group ──────────────────────────────────────────────────────
    let provider_group = adw::PreferencesGroup::builder()
        .title("LLM Provider")
        .build();

    let provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Groq", "Gemini", "Custom"]))
        .build();

    let cfg = saver.config.borrow();
    provider_row.set_selected(postprocess_provider_idx(&cfg.postprocess_provider));

    let api_key_row = adw::PasswordEntryRow::builder()
        .title("API Key")
        .text(cfg.active_postprocess_key())
        .build();

    let model_row = adw::EntryRow::builder()
        .title("Model")
        .text(cfg.active_postprocess_model())
        .build();

    let url_row = adw::EntryRow::builder()
        .title("API URL")
        .text(&cfg.postprocess_custom_api_url)
        .visible(cfg.postprocess_provider == "custom")
        .build();

    drop(cfg);

    provider_row.connect_selected_notify({
        let saver = saver.clone();
        let api_key_row = api_key_row.clone();
        let model_row = model_row.clone();
        let url_row = url_row.clone();
        move |row| {
            {
                let mut cfg = saver.config.borrow_mut();
                let key   = api_key_row.text().to_string();
                let model = model_row.text().to_string();
                let url   = url_row.text().to_string();
                match cfg.postprocess_provider.as_str() {
                    "groq" => {
                        cfg.postprocess_groq_api_key = key;
                        cfg.postprocess_groq_model   = model;
                    }
                    "gemini" => {
                        cfg.postprocess_gemini_api_key = key;
                        cfg.postprocess_gemini_model   = model;
                    }
                    _ => {
                        cfg.postprocess_custom_api_key = key;
                        cfg.postprocess_custom_model   = model;
                        cfg.postprocess_custom_api_url = url;
                    }
                }
                cfg.postprocess_provider =
                    postprocess_provider_str(row.selected()).to_string();
            }
            let (new_key, new_model, new_url) = {
                let c = saver.config.borrow();
                (
                    c.active_postprocess_key().to_string(),
                    c.active_postprocess_model().to_string(),
                    c.postprocess_custom_api_url.clone(),
                )
            };
            api_key_row.set_text(&new_key);
            model_row.set_text(&new_model);
            url_row.set_text(&new_url);
            url_row.set_visible(postprocess_provider_str(row.selected()) == "custom");
            saver.save();
        }
    });

    api_key_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(|cfg| match cfg.postprocess_provider.as_str() {
                "groq"   => cfg.postprocess_groq_api_key   = t,
                "gemini" => cfg.postprocess_gemini_api_key = t,
                _        => cfg.postprocess_custom_api_key = t,
            });
        }
    });

    model_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(|cfg| match cfg.postprocess_provider.as_str() {
                "groq"   => cfg.postprocess_groq_model   = t,
                "gemini" => cfg.postprocess_gemini_model = t,
                _        => cfg.postprocess_custom_model = t,
            });
        }
    });

    url_row.connect_changed({
        let saver = saver.clone();
        move |row| saver.update(|cfg| cfg.postprocess_custom_api_url = row.text().to_string())
    });

    provider_group.add(&provider_row);
    provider_group.add(&api_key_row);
    provider_group.add(&model_row);
    provider_group.add(&url_row);

    // ── Prompt group ────────────────────────────────────────────────────────
    let prompt_group = adw::PreferencesGroup::builder()
        .title("System Prompt")
        .description("Sent to the LLM along with the raw transcript.")
        .build();

    let prompt_buffer = gtk4::TextBuffer::new(None);
    prompt_buffer.set_text(&saver.config.borrow().postprocess_prompt);

    let prompt_view = gtk4::TextView::builder()
        .buffer(&prompt_buffer)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();

    let prompt_scroll = gtk4::ScrolledWindow::builder()
        .height_request(160)
        .child(&prompt_view)
        .build();

    let prompt_frame = gtk4::Frame::builder()
        .child(&prompt_scroll)
        .build();
    prompt_frame.add_css_class("card");

    prompt_buffer.connect_changed({
        let saver = saver.clone();
        move |buf| {
            let t = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            saver.update(|cfg| cfg.postprocess_prompt = t);
        }
    });

    let reset_btn = gtk4::Button::builder()
        .label("Reset to Default")
        .halign(gtk4::Align::End)
        .build();
    reset_btn.connect_clicked({
        let prompt_buffer = prompt_buffer.clone();
        move |_| {
            prompt_buffer.set_text(config::DEFAULT_POSTPROCESS_PROMPT);
        }
    });

    let prompt_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    prompt_box.append(&prompt_frame);
    prompt_box.append(&reset_btn);
    prompt_group.add(&prompt_box);

    // ── Assemble ────────────────────────────────────────────────────────────
    page.add(&enable_group);
    page.add(&provider_group);
    page.add(&prompt_group);
    (page, enable_row)
}

// ── History ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct HistoryEntry {
    id:            i64,
    timestamp:     i64,
    text:          String,
    status:        String,
    error:         Option<String>,
    wav_path:      Option<String>,
    text_original: Option<String>,
}

fn db_path() -> Option<std::path::PathBuf> {
    Some(config::data_dir().join("history.db"))
}

fn latest_history_id() -> i64 {
    let Some(path) = db_path() else { return 0 };
    if !path.exists() { return 0; }
    let Ok(conn) = Connection::open(&path) else { return 0 };
    conn.query_row("SELECT COALESCE(MAX(id), 0) FROM history", [], |row| row.get(0))
        .unwrap_or(0)
}

fn load_history() -> Vec<HistoryEntry> {
    let Some(path) = db_path() else { return vec![] };
    if !path.exists() { return vec![]; }
    let Ok(conn) = Connection::open(&path) else { return vec![] };

    for sql in &[
        "ALTER TABLE history ADD COLUMN status        TEXT NOT NULL DEFAULT 'ok'",
        "ALTER TABLE history ADD COLUMN error         TEXT",
        "ALTER TABLE history ADD COLUMN wav_path      TEXT",
        "ALTER TABLE history ADD COLUMN text_original TEXT",
    ] {
        let _ = conn.execute(sql, []);
    }

    let Ok(mut stmt) = conn.prepare(
        "SELECT id, timestamp, text, status, error, wav_path, text_original
         FROM history ORDER BY id DESC LIMIT 200",
    ) else {
        return vec![];
    };

    stmt.query_map([], |row| {
        Ok(HistoryEntry {
            id:            row.get(0)?,
            timestamp:     row.get(1)?,
            text:          row.get(2)?,
            status:        row.get(3)?,
            error:         row.get(4)?,
            wav_path:      row.get(5)?,
            text_original: row.get(6)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

fn delete_history_entry(id: i64, wav_path: Option<&str>) {
    if let Some(p) = wav_path {
        let _ = std::fs::remove_file(p);
    }
    let Some(db) = db_path() else { return };
    if let Ok(conn) = Connection::open(&db) {
        let _ = conn.execute("DELETE FROM history WHERE id = ?1", params![id]);
    }
}

fn clear_history_db() {
    let Some(db) = db_path() else { return };
    if let Ok(conn) = Connection::open(&db) {
        if let Ok(mut stmt) =
            conn.prepare("SELECT wav_path FROM history WHERE wav_path IS NOT NULL")
        {
            if let Ok(paths) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for p in paths.flatten() {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
        let _ = conn.execute("DELETE FROM history", []);
    }
}

fn wav_duration_secs(path: &str) -> Option<f64> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 44];
    f.read_exact(&mut header).ok()?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return None;
    }
    let channels        = u16::from_le_bytes([header[22], header[23]]) as u32;
    let sample_rate     = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    let bits_per_sample = u16::from_le_bytes([header[34], header[35]]) as u32;
    let data_size       = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
    if sample_rate == 0 || channels == 0 || bits_per_sample == 0 {
        return None;
    }
    let bytes_per_second = sample_rate * channels * (bits_per_sample / 8);
    Some(data_size as f64 / bytes_per_second as f64)
}

fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let mins = secs as u64 / 60;
        let remainder = secs as u64 % 60;
        format!("{}m {:02}s", mins, remainder)
    }
}

fn build_history_page(toast: &adw::ToastOverlay) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let stack = gtk4::Stack::new();

    let empty = adw::StatusPage::builder()
        .icon_name("document-open-recent-symbolic")
        .title("No Transcriptions Yet")
        .description("Your transcription history will appear here")
        .vexpand(true)
        .build();
    stack.add_named(&empty, Some("empty"));

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();
    let list_box = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(vec!["boxed-list"])
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    scrolled.set_child(Some(&list_box));
    stack.add_named(&scrolled, Some("list"));

    let refresh_slot: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    // (timestamp_unix_secs, row, cached " · <duration>" suffix or "")
    let row_tracker: Rc<RefCell<Vec<(i64, adw::ActionRow, String)>>> =
        Rc::new(RefCell::new(Vec::new()));

    let refresh: Rc<dyn Fn()> = {
        let list_box     = list_box.clone();
        let stack        = stack.clone();
        let toast        = toast.clone();
        let refresh_slot = Rc::clone(&refresh_slot);
        let row_tracker  = Rc::clone(&row_tracker);
        Rc::new(move || {
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            row_tracker.borrow_mut().clear();
            let history = load_history();
            if history.is_empty() {
                stack.set_visible_child_name("empty");
            } else {
                stack.set_visible_child_name("list");
                let refresh = refresh_slot.borrow().clone().unwrap();
                for entry in history {
                    let ts = entry.timestamp;
                    let (row, duration_suffix) =
                        build_history_row(entry, &refresh, &toast);
                    list_box.append(&row);
                    row_tracker.borrow_mut().push((ts, row, duration_suffix));
                }
            }
        })
    };

    *refresh_slot.borrow_mut() = Some(Rc::clone(&refresh));
    refresh();

    {
        let row_tracker = Rc::clone(&row_tracker);
        glib::timeout_add_seconds_local(30, move || {
            for (ts, row, suffix) in row_tracker.borrow().iter() {
                row.set_subtitle(&format!("{}{}", format_timestamp(*ts), suffix));
            }
            glib::ControlFlow::Continue
        });
    }

    (stack.upcast(), refresh)
}

fn build_history_row(
    entry:   HistoryEntry,
    refresh: &Rc<dyn Fn()>,
    toast:   &adw::ToastOverlay,
) -> (adw::ActionRow, String) {
    let failed = entry.status == "failed";

    let title = if failed {
        let msg = entry.error.as_deref().unwrap_or("unknown error");
        format!("Failed: {}", glib::markup_escape_text(msg))
    } else {
        glib::markup_escape_text(&entry.text).into()
    };

    let duration_suffix = entry
        .wav_path
        .as_ref()
        .and_then(|wav| wav_duration_secs(wav))
        .map(|dur| format!(" \u{00B7} {}", format_duration(dur)))
        .unwrap_or_default();
    let subtitle = format!("{}{}", format_timestamp(entry.timestamp), duration_suffix);

    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();

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
    } else if let Some(raw) = entry.text_original.as_deref() {
        if raw != entry.text {
            // Polished entry: a toggle that swaps the visible title between the
            // polished text and the original (unpolished) transcript.
            let polished = glib::markup_escape_text(&entry.text).to_string();
            let original = glib::markup_escape_text(raw).to_string();
            let polished_raw = entry.text.clone();
            let original_raw = raw.to_string();

            let toggle = gtk4::ToggleButton::builder()
                .icon_name("starred-symbolic")
                .valign(gtk4::Align::Center)
                .css_classes(vec!["flat"])
                .tooltip_text("Show original (unpolished) transcript")
                .build();

            toggle.connect_toggled({
                let row = row.clone();
                let displayed = Rc::clone(&displayed);
                move |t| {
                    if t.is_active() {
                        row.set_title(&original);
                        *displayed.borrow_mut() = original_raw.clone();
                        t.set_icon_name("non-starred-symbolic");
                        t.set_tooltip_text(Some("Show polished transcript"));
                    } else {
                        row.set_title(&polished);
                        *displayed.borrow_mut() = polished_raw.clone();
                        t.set_icon_name("starred-symbolic");
                        t.set_tooltip_text(Some("Show original (unpolished) transcript"));
                    }
                }
            });
            row.add_prefix(&toggle);
        }
    }

    // Copy button for successful entries
    if !failed {
        let copy_btn = gtk4::Button::builder()
            .icon_name("edit-copy-symbolic")
            .valign(gtk4::Align::Center)
            .css_classes(vec!["flat"])
            .tooltip_text("Copy to clipboard")
            .build();
        copy_btn.connect_clicked({
            let displayed = Rc::clone(&displayed);
            let toast = toast.clone();
            move |_| {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(&displayed.borrow());
                }
                toast.add_toast(
                    adw::Toast::builder().title("Copied").timeout(2).build(),
                );
            }
        });
        row.add_suffix(&copy_btn);
    }

    // Retry button for failed entries with saved audio
    if failed {
        if let Some(ref wav_path) = entry.wav_path {
            let spinner = adw::Spinner::new();
            spinner.set_visible(false);
            spinner.set_valign(gtk4::Align::Center);

            let retry_btn = gtk4::Button::builder()
                .label("Retry")
                .valign(gtk4::Align::Center)
                .css_classes(vec!["suggested-action"])
                .build();

            row.add_suffix(&spinner);
            row.add_suffix(&retry_btn);

            retry_btn.connect_clicked({
                let spinner   = spinner.clone();
                let retry_btn = retry_btn.clone();
                let refresh   = Rc::clone(refresh);
                let id        = entry.id;
                let wav       = wav_path.clone();

                move |_| {
                    retry_btn.set_visible(false);
                    spinner.set_visible(true);

                    let Ok(conn) = gio::bus_get_sync(
                        gio::BusType::Session,
                        gio::Cancellable::NONE,
                    ) else {
                        spinner.set_visible(false);
                        retry_btn.set_visible(true);
                        return;
                    };

                    let params     = (id, wav.as_str()).to_variant();
                    let spinner2   = spinner.clone();
                    let retry_btn2 = retry_btn.clone();
                    let refresh2   = Rc::clone(&refresh);

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
                            spinner2.set_visible(false);
                            match result {
                                Ok(_) => refresh2(),
                                Err(_) => {
                                    retry_btn2.set_visible(true);
                                    refresh2();
                                }
                            }
                        },
                    );
                }
            });
        }
    }

    // Play button for entries with saved audio
    if let Some(ref wav_path) = entry.wav_path {
        if std::path::Path::new(wav_path).exists() {
            let play_btn = gtk4::Button::builder()
                .icon_name("media-playback-start-symbolic")
                .valign(gtk4::Align::Center)
                .css_classes(vec!["flat"])
                .tooltip_text("Play recording")
                .build();

            let media_file: Rc<RefCell<Option<gtk4::MediaFile>>> =
                Rc::new(RefCell::new(None));

            play_btn.connect_clicked({
                let wav_path = wav_path.clone();
                let play_btn = play_btn.clone();
                let media_file = Rc::clone(&media_file);
                move |_| {
                    let mut mf_slot = media_file.borrow_mut();

                    if let Some(ref mf) = *mf_slot {
                        mf.set_playing(false);
                        play_btn.set_icon_name("media-playback-start-symbolic");
                        play_btn.set_tooltip_text(Some("Play recording"));
                        *mf_slot = None;
                        return;
                    }

                    let file = gio::File::for_path(&wav_path);
                    let mf = gtk4::MediaFile::for_file(&file);

                    mf.connect_ended_notify({
                        let play_btn = play_btn.clone();
                        let media_file = Rc::clone(&media_file);
                        move |mf| {
                            if mf.is_ended() {
                                play_btn.set_icon_name(
                                    "media-playback-start-symbolic",
                                );
                                play_btn
                                    .set_tooltip_text(Some("Play recording"));
                                *media_file.borrow_mut() = None;
                            }
                        }
                    });

                    mf.play();
                    play_btn.set_icon_name("media-playback-stop-symbolic");
                    play_btn.set_tooltip_text(Some("Stop playback"));
                    *mf_slot = Some(mf);
                }
            });

            row.add_suffix(&play_btn);
        }
    }

    // Delete button
    let del_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk4::Align::Center)
        .css_classes(vec!["flat"])
        .tooltip_text("Delete")
        .build();
    del_btn.connect_clicked({
        let refresh = Rc::clone(refresh);
        let id      = entry.id;
        let wav     = entry.wav_path.clone();
        move |_| {
            delete_history_entry(id, wav.as_deref());
            refresh();
        }
    });
    row.add_suffix(&del_btn);

    (row, duration_suffix)
}

fn format_timestamp(ts: i64) -> String {
    if ts == 0 {
        return String::new();
    }
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH + Duration::from_secs(ts as u64))
        .unwrap_or_default()
        .as_secs();
    match elapsed {
        0..=59       => "just now".into(),
        60..=3599    => format!("{} min ago", elapsed / 60),
        3600..=86399 => format!("{} hr ago", elapsed / 3600),
        s            => format!("{} days ago", s / 86400),
    }
}

// ── Status page ──────────────────────────────────────────────────────────────

enum FixAction {
    RunCommand {
        btn_label: &'static str,
        _cmd:      &'static str,
        program:   &'static str,
        args:      &'static [&'static str],
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

/// True if the session is GNOME Shell (where the hotkey/VU meter come from the
/// GNOME extension). On Hyprland / Sway / KDE / other wlroots compositors this
/// is false and the standalone overlay binary plays that role instead.
fn is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_ascii_lowercase().contains("gnome"))
        .unwrap_or(false)
}

/// Whether `name` is installed, checking both the install.sh location
/// (~/.local/bin) and anywhere on `$PATH`. The PATH check covers NixOS /
/// home-manager, where binaries live in the read-only Nix store rather than
/// ~/.local/bin.
fn binary_available(name: &str, local_subpath: &str) -> bool {
    if dirs::home_dir()
        .map(|h| h.join(local_subpath).exists())
        .unwrap_or(false)
    {
        return true;
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|p| p.join(name).exists()))
        .unwrap_or(false)
}

fn service_active(service: &str) -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", service])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn check_daemon_installed() -> bool {
    binary_available(
        "transcriber-daemon",
        ".local/bin/transcriber-daemon",
    )
}

fn check_daemon_running() -> bool {
    service_active("transcriber-daemon.service")
}

fn check_overlay_installed() -> bool {
    binary_available(
        "transcriber-overlay",
        ".local/bin/transcriber-overlay",
    )
}

fn check_overlay_running() -> bool {
    service_active("transcriber-overlay.service")
}

fn check_extension_installed() -> bool {
    dirs::data_local_dir()
        .map(|d| d.join("gnome-shell/extensions/transcriber@local").exists())
        .unwrap_or(false)
}

fn check_extension_loaded() -> bool {
    std::process::Command::new("gnome-extensions")
        .args(["list"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("transcriber@local"))
        .unwrap_or(false)
}

fn check_extension_enabled() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.shell", "enabled-extensions"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("transcriber@local"))
        .unwrap_or(false)
}

fn check_api_key() -> bool {
    config::load()
        .map(|c| !c.active_key().is_empty())
        .unwrap_or(false)
}

const DAEMON_INSTALLED_CHECK: StatusCheck = StatusCheck {
    title:    "Daemon installed",
    ok_msg:   "Binary found in ~/.local/bin or on $PATH",
    fail_msg: "Binary not found — run install.sh (or rebuild your Nix config)",
    run:      check_daemon_installed,
    fix:      Some(FixAction::CopyText {
        btn_label: "Copy command",
        text:      "bash install.sh",
    }),
};

const DAEMON_RUNNING_CHECK: StatusCheck = StatusCheck {
    title:    "Daemon running",
    ok_msg:   "systemd service is active",
    fail_msg: "Service not running",
    run:      check_daemon_running,
    fix:      Some(FixAction::RunCommand {
        btn_label: "Start daemon",
        _cmd:      "systemctl --user enable --now transcriber-daemon.service",
        program:   "systemctl",
        args:      &["--user", "enable", "--now", "transcriber-daemon.service"],
    }),
};

const API_KEY_CHECK: StatusCheck = StatusCheck {
    title:    "API key configured",
    ok_msg:   "API key is set",
    fail_msg: "No API key — transcription will fail",
    run:      check_api_key,
    fix:      Some(FixAction::GoToSettings),
};

/// Checks shown on GNOME Shell sessions: the hotkey + VU meter come from the
/// GNOME extension.
const GNOME_CHECKS: &[StatusCheck] = &[
    DAEMON_INSTALLED_CHECK,
    DAEMON_RUNNING_CHECK,
    StatusCheck {
        title:    "Extension installed",
        ok_msg:   "Found in ~/.local/share/gnome-shell/extensions/",
        fail_msg: "Extension not found — run install.sh",
        run:      check_extension_installed,
        fix:      Some(FixAction::CopyText {
            btn_label: "Copy command",
            text:      "bash install.sh",
        }),
    },
    StatusCheck {
        title:    "Extension loaded by GNOME Shell",
        ok_msg:   "GNOME Shell has scanned the extension",
        fail_msg: "Not loaded yet — log out and back in on Wayland",
        run:      check_extension_loaded,
        fix:      Some(FixAction::CopyText {
            btn_label: "Copy logout command",
            text:      "gnome-session-quit --logout",
        }),
    },
    StatusCheck {
        title:    "Extension enabled",
        ok_msg:   "Enabled in GNOME Shell",
        fail_msg: "Extension loaded but not enabled",
        run:      check_extension_enabled,
        fix:      Some(FixAction::RunCommand {
            btn_label: "Enable extension",
            _cmd:      "gnome-extensions enable transcriber@local",
            program:   "gnome-extensions",
            args:      &["enable", "transcriber@local"],
        }),
    },
    API_KEY_CHECK,
];

/// Checks shown on wlroots compositors (Hyprland / Sway / KDE): the hotkey +
/// VU meter come from the standalone overlay binary, not a GNOME extension.
const WLROOTS_CHECKS: &[StatusCheck] = &[
    DAEMON_INSTALLED_CHECK,
    DAEMON_RUNNING_CHECK,
    StatusCheck {
        title:    "Overlay installed",
        ok_msg:   "Binary found in ~/.local/bin or on $PATH",
        fail_msg: "Overlay not found — run install.sh (or enable it in your Nix config)",
        run:      check_overlay_installed,
        fix:      Some(FixAction::CopyText {
            btn_label: "Copy command",
            text:      "bash install.sh",
        }),
    },
    StatusCheck {
        title:    "Overlay running",
        ok_msg:   "systemd service is active",
        fail_msg: "Overlay not running",
        run:      check_overlay_running,
        fix:      Some(FixAction::RunCommand {
            btn_label: "Start overlay",
            _cmd:      "systemctl --user enable --now transcriber-overlay.service",
            program:   "systemctl",
            args:      &["--user", "enable", "--now", "transcriber-overlay.service"],
        }),
    },
    API_KEY_CHECK,
];

fn active_checks() -> &'static [StatusCheck] {
    if is_gnome() {
        GNOME_CHECKS
    } else {
        WLROOTS_CHECKS
    }
}

fn apply_check_state(
    row:  &adw::ActionRow,
    icon: &gtk4::Image,
    btn:  &gtk4::Button,
    ok:   bool,
    check: &StatusCheck,
) {
    if ok {
        row.set_subtitle(check.ok_msg);
        icon.set_icon_name(Some("object-select-symbolic"));
        icon.remove_css_class("warning");
        icon.add_css_class("success");
        btn.set_visible(false);
    } else {
        row.set_subtitle(check.fail_msg);
        icon.set_icon_name(Some("dialog-warning-symbolic"));
        icon.remove_css_class("success");
        icon.add_css_class("warning");
        btn.set_visible(true);
    }
}

fn build_status_page(
    toast:      &adw::ToastOverlay,
    view_stack: &adw::ViewStack,
) -> (adw::PreferencesPage, Rc<dyn Fn()>) {
    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .title("Setup")
        .description("Verifies installation and configuration")
        .build();

    let mut refreshers: Vec<Box<dyn Fn()>> = Vec::new();

    for check in active_checks() {
        let row = adw::ActionRow::builder().title(check.title).build();

        let icon = gtk4::Image::new();
        icon.set_pixel_size(16);
        icon.set_valign(gtk4::Align::Center);
        row.add_prefix(&icon);

        let spinner = adw::Spinner::new();
        spinner.set_visible(false);
        spinner.set_valign(gtk4::Align::Center);
        row.add_suffix(&spinner);

        let fix_btn = gtk4::Button::builder()
            .valign(gtk4::Align::Center)
            .css_classes(vec!["suggested-action"])
            .visible(false)
            .build();

        if let Some(ref fix) = check.fix {
            match fix {
                FixAction::RunCommand { btn_label, program, args, .. } => {
                    fix_btn.set_label(btn_label);
                    let program = *program;
                    let args: &'static [&str] = *args;

                    fix_btn.connect_clicked({
                        let row     = row.clone();
                        let icon    = icon.clone();
                        let spinner = spinner.clone();
                        let fix_btn = fix_btn.clone();
                        let toast   = toast.clone();
                        move |_| {
                            fix_btn.set_visible(false);
                            spinner.set_visible(true);

                            let (tx, rx) = std::sync::mpsc::sync_channel(1);
                            std::thread::spawn(move || {
                                let _ = tx.send(
                                    std::process::Command::new(program)
                                        .args(args)
                                        .output(),
                                );
                            });

                            let row2     = row.clone();
                            let icon2    = icon.clone();
                            let spinner2 = spinner.clone();
                            let fix_btn2 = fix_btn.clone();
                            let toast2   = toast.clone();

                            glib::idle_add_local(move || {
                                match rx.try_recv() {
                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                        return glib::ControlFlow::Continue;
                                    }
                                    Ok(Ok(out)) if out.status.success() => {
                                        spinner2.set_visible(false);
                                        apply_check_state(
                                            &row2, &icon2, &fix_btn2,
                                            (check.run)(), check,
                                        );
                                    }
                                    Ok(Ok(out)) => {
                                        spinner2.set_visible(false);
                                        fix_btn2.set_visible(true);
                                        let code = out.status.code().unwrap_or(-1);
                                        toast2.add_toast(
                                            adw::Toast::builder()
                                                .title(&format!(
                                                    "Command failed (exit {code})"
                                                ))
                                                .timeout(4)
                                                .build(),
                                        );
                                    }
                                    _ => {
                                        spinner2.set_visible(false);
                                        fix_btn2.set_visible(true);
                                    }
                                }
                                glib::ControlFlow::Break
                            });
                        }
                    });
                }

                FixAction::CopyText { btn_label, text } => {
                    fix_btn.set_label(btn_label);
                    fix_btn.set_tooltip_text(Some(&format!("Copy: {text}")));
                    let text = *text;

                    fix_btn.connect_clicked({
                        let toast = toast.clone();
                        move |_| {
                            if let Some(display) = gtk4::gdk::Display::default() {
                                display.clipboard().set_text(text);
                            }
                            toast.add_toast(
                                adw::Toast::builder()
                                    .title("Copied to clipboard")
                                    .timeout(2)
                                    .build(),
                            );
                        }
                    });
                }

                FixAction::GoToSettings => {
                    fix_btn.set_label("Go to Settings");
                    fix_btn.connect_clicked({
                        let vs = view_stack.clone();
                        move |_| vs.set_visible_child_name("settings")
                    });
                }
            }
        }

        row.add_suffix(&fix_btn);
        apply_check_state(&row, &icon, &fix_btn, (check.run)(), check);

        refreshers.push({
            let row     = row.clone();
            let icon    = icon.clone();
            let fix_btn = fix_btn.clone();
            Box::new(move || {
                apply_check_state(&row, &icon, &fix_btn, (check.run)(), check)
            })
        });

        group.add(&row);
    }

    // Recheck button
    let recheck_group = adw::PreferencesGroup::new();
    let recheck = adw::ButtonRow::builder()
        .title("Recheck All")
        .start_icon_name("view-refresh-symbolic")
        .build();

    let refresh_all: Rc<dyn Fn()> = {
        let fns = Rc::new(refreshers);
        Rc::new(move || {
            for f in fns.iter() {
                f();
            }
        })
    };

    recheck.connect_activated({
        let r = Rc::clone(&refresh_all);
        move |_| r()
    });
    recheck_group.add(&recheck);

    page.add(&group);
    page.add(&recheck_group);

    (page, refresh_all)
}
