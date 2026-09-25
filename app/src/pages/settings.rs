use std::rc::Rc;

use common::config;
use gtk4::gio;
use libadwaita as adw;
use adw::prelude::*;

use crate::ext_settings::is_gnome;
use crate::saver::AutoSaver;
use crate::ui;

/// Widgets on this page whose behaviour is coupled to other pages; the
/// coupling itself lives in `pages::coupling`.
pub struct SettingsPage {
    pub page:               adw::PreferencesPage,
    pub streaming_expander: adw::ExpanderRow,
    pub direct_inject_row:  adw::SwitchRow,
    /// Re-evaluates whether the vocabulary controls do anything. Depends on
    /// both the transcription provider and the post-processing switch.
    pub sync_vocab:         Rc<dyn Fn()>,
}

fn provider_idx(p: &str) -> u32 {
    match p { "cohere" => 1, "custom" => 2, _ => 0 }
}

fn provider_str(idx: u32) -> &'static str {
    match idx { 1 => "cohere", 2 => "custom", _ => "groq" }
}

const KEYBOARD_MODES: [&str; 3] = ["none", "ondemand", "exclusive"];

pub fn build(
    saver: &AutoSaver,
    ext_settings: &Option<gio::Settings>,
    toast: &adw::ToastOverlay,
) -> SettingsPage {
    let page = adw::PreferencesPage::new();

    let (vocab_group, sync_vocab) = build_vocab_group(saver, toast);
    let provider_group = build_provider_group(saver, &sync_vocab);
    let (behaviour_group, direct_inject_row) = build_behaviour_group(saver, ext_settings);
    let (streaming_group, streaming_expander) = build_streaming_group(saver);
    let history_group = build_history_group(saver);
    let audio_group = build_audio_group(saver);

    page.add(&provider_group);
    page.add(&vocab_group);
    page.add(&behaviour_group);
    page.add(&streaming_group);
    page.add(&history_group);
    page.add(&audio_group);
    // Keyboard mode and VU-meter tuning are read by the wlroots overlay only;
    // the GNOME extension has its own meter and never takes focus.
    if !is_gnome() {
        page.add(&build_overlay_group(saver));
    }

    SettingsPage { page, streaming_expander, direct_inject_row, sync_vocab }
}

// ── Provider ─────────────────────────────────────────────────────────────────

fn build_provider_group(saver: &AutoSaver, sync_vocab: &Rc<dyn Fn()>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Transcription Provider")
        .build();

    let cfg = saver.config();
    let provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Groq", "Cohere", "Custom"]))
        .build();
    // Set after construction: builder properties have no guaranteed order,
    // and `selected` is clamped against whatever model is present at the time.
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

    // One handler for everything a provider switch affects. The key/model rows
    // have already written their text to the outgoing provider's fields as the
    // user typed, so switching only has to repoint the rows.
    provider_row.connect_selected_notify({
        let saver = saver.clone();
        let api_key_row = api_key_row.clone();
        let model_row = model_row.clone();
        let url_row = url_row.clone();
        let sync_vocab = Rc::clone(sync_vocab);
        move |row| {
            let provider = provider_str(row.selected());
            saver.update(move |cfg| cfg.provider = provider.to_string());
            let (key, model) = {
                let c = saver.config();
                (c.active_key().to_string(), c.active_model().to_string())
            };
            api_key_row.set_text(&key);
            model_row.set_text(&model);
            url_row.set_visible(provider == "custom");
            sync_vocab();
        }
    });

    api_key_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| match cfg.provider.as_str() {
                "groq"   => cfg.groq_api_key   = t.clone(),
                "cohere" => cfg.cohere_api_key = t.clone(),
                _        => cfg.custom_api_key = t.clone(),
            });
        }
    });

    model_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| match cfg.provider.as_str() {
                "groq"   => cfg.groq_model   = t.clone(),
                "cohere" => cfg.cohere_model = t.clone(),
                _        => cfg.custom_model = t.clone(),
            });
        }
    });

    url_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| cfg.custom_api_url = t.clone());
        }
    });

    lang_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| {
                cfg.language = if t.is_empty() { None } else { Some(t.clone()) }
            });
        }
    });

    group.add(&provider_row);
    group.add(&api_key_row);
    group.add(&model_row);
    group.add(&url_row);
    group.add(&lang_row);
    group
}

// ── Vocabulary ───────────────────────────────────────────────────────────────

fn build_vocab_group(
    saver: &AutoSaver,
    toast: &adw::ToastOverlay,
) -> (adw::PreferencesGroup, Rc<dyn Fn()>) {
    let group = adw::PreferencesGroup::builder()
        .title("Vocabulary")
        .description(
            "Product names, proper nouns and jargon you want spelled exactly — \
             one term per line. Sent to Whisper as a spelling hint, and to the \
             post-processing LLM as the authoritative spellings.",
        )
        .build();

    let enable_row = adw::SwitchRow::builder()
        .title("Use vocabulary")
        .subtitle("Omits the terms from both transcription and post-processing when off")
        .active(saver.config().vocabulary_enabled)
        .build();
    enable_row.connect_active_notify({
        let saver = saver.clone();
        move |row| {
            let on = row.is_active();
            saver.update(move |cfg| cfg.vocabulary_enabled = on);
        }
    });

    let buffer = gtk4::TextBuffer::new(None);
    buffer.set_text(&saver.config().vocabulary.join("\n"));

    let view = gtk4::TextView::builder()
        .buffer(&buffer)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();
    // A bare TextView has no name for screen readers; the group title sits
    // outside it and isn't associated automatically.
    view.update_property(&[gtk4::accessible::Property::Label("Vocabulary terms, one per line")]);

    let scroll = gtk4::ScrolledWindow::builder()
        .height_request(140)
        .child(&view)
        .build();
    let frame = gtk4::Frame::builder().child(&scroll).build();
    frame.add_css_class("card");

    // Live budget readout. Whisper drops everything past 224 tokens without
    // reporting it, so the only way a user learns their last few terms stopped
    // working is if the GUI says so before the request is ever sent.
    let counter = gtk4::Label::builder().halign(gtk4::Align::Start).build();
    counter.add_css_class("caption");
    counter.add_css_class("dim-label");

    let update_counter = {
        let counter = counter.clone();
        move |terms: &[String]| {
            let tokens = config::estimate_prompt_tokens(terms);
            let limit = config::VOCABULARY_TOKEN_LIMIT;
            counter.set_label(&format!(
                "{} term{} · ~{tokens}/{limit} tokens",
                terms.len(),
                if terms.len() == 1 { "" } else { "s" },
            ));
            if tokens > limit {
                counter.remove_css_class("dim-label");
                counter.add_css_class("error");
                counter.set_tooltip_text(Some(
                    "Over Whisper's 224-token cap — terms past the limit are \
                     dropped from the transcription hint (post-processing still \
                     gets them all). Estimate is deliberately high.",
                ));
            } else {
                counter.remove_css_class("error");
                counter.add_css_class("dim-label");
                counter.set_tooltip_text(None);
            }
        }
    };
    update_counter(&saver.config().vocabulary);

    buffer.connect_changed({
        let saver = saver.clone();
        move |buf| {
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            let terms: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            update_counter(&terms);
            saver.update(move |cfg| cfg.vocabulary = terms.clone());
        }
    });

    // Shown only for Cohere + post-processing, where half of what the group
    // description promises doesn't apply.
    let cohere_note = gtk4::Label::builder()
        .label(
            "Cohere ignores the Whisper spelling hint, but post-processing \
             still uses these terms.",
        )
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .build();
    cohere_note.add_css_class("caption");
    cohere_note.add_css_class("dim-label");

    let reset = gtk4::Button::builder()
        .label("Reset to Default")
        .halign(gtk4::Align::End)
        .build();
    reset.connect_clicked({
        let buffer = buffer.clone();
        let toast = toast.clone();
        move |_| {
            ui::reset_buffer_with_undo(&toast, &buffer, &config::DEFAULT_VOCABULARY.join("\n"), "Vocabulary")
        }
    });

    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    vbox.append(&frame);
    vbox.append(&counter);
    vbox.append(&cohere_note);
    vbox.append(&reset);

    group.add(&enable_row);
    group.add(&vbox);

    // Cohere's endpoint documents no `prompt` field, so `active_prompt()`
    // withholds the hint there. But `active_postprocess_system_prompt()` sends
    // the list to the polish LLM regardless of provider — so the controls only
    // do nothing at all when Cohere is selected *and* post-processing is off.
    let sync: Rc<dyn Fn()> = {
        let saver = saver.clone();
        let group = group.clone();
        Rc::new(move || {
            let (cohere, postprocess) = {
                let c = saver.config();
                (c.provider == "cohere", c.postprocess_enabled)
            };
            let useful = !cohere || postprocess;
            group.set_sensitive(useful);
            group.set_tooltip_text(if useful {
                None
            } else {
                Some(
                    "Cohere's transcription endpoint accepts no vocabulary hint. \
                     Turn on post-processing to use these terms there instead.",
                )
            });
            cohere_note.set_visible(cohere && postprocess);
        })
    };
    sync();
    (group, sync)
}

// ── Behaviour ────────────────────────────────────────────────────────────────

fn build_behaviour_group(
    saver: &AutoSaver,
    ext_settings: &Option<gio::Settings>,
) -> (adw::PreferencesGroup, adw::SwitchRow) {
    let group = adw::PreferencesGroup::builder().title("Behaviour").build();

    // Auto-paste is implemented by the GNOME extension; on wlroots the overlay
    // delivers text itself, so the row would be a control for nothing.
    if is_gnome() {
        let paste_row = adw::SwitchRow::builder()
            .title("Auto-paste after transcription")
            .subtitle("Pastes into the focused window")
            .build();
        match ext_settings {
            // A binding (rather than a one-off read) keeps the row in step with
            // changes made from the extension's own preferences.
            Some(s) => s.bind("auto-paste", &paste_row, "active").build(),
            None => {
                paste_row.set_sensitive(false);
                paste_row.set_subtitle("GNOME extension not installed");
            }
        }
        group.add(&paste_row);
    }

    let vad_row = adw::SwitchRow::builder()
        .title("Auto-stop on silence")
        .subtitle("Stop recording after ~1.5 s of silence")
        .active(saver.config().vad_enabled)
        .build();
    vad_row.connect_active_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.is_active();
            saver.update(move |cfg| cfg.vad_enabled = v);
        }
    });

    let max_rec_row = adw::SpinRow::builder()
        .title("Maximum recording length")
        .subtitle("Seconds before recording stops on its own; 0 for no limit")
        .adjustment(&gtk4::Adjustment::new(
            saver.config().max_recording_secs as f64,
            0.0,
            7200.0,
            30.0,
            300.0,
            0.0,
        ))
        .build();
    max_rec_row.connect_value_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.value() as u32;
            saver.update(move |cfg| cfg.max_recording_secs = v);
        }
    });

    // Its handler lives in `pages::coupling`, alongside streaming, which
    // depends on it.
    let direct_inject_row = adw::SwitchRow::builder()
        .title("Direct text injection")
        .subtitle("Type text directly instead of using the clipboard")
        .active(saver.config().direct_injection)
        .build();

    let audio_cues_row = adw::SwitchRow::builder()
        .title("Audio feedback cues")
        .subtitle("Sounds on recording start and transcription finish")
        .active(saver.config().audio_cues_enabled)
        .build();
    audio_cues_row.connect_active_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.is_active();
            saver.update(move |cfg| cfg.audio_cues_enabled = v);
        }
    });

    group.add(&vad_row);
    group.add(&max_rec_row);
    group.add(&direct_inject_row);
    group.add(&audio_cues_row);
    (group, direct_inject_row)
}

// ── Streaming ────────────────────────────────────────────────────────────────

fn streaming_prov_idx(p: &str) -> u32 {
    if p == "assemblyai" { 1 } else { 0 }
}

fn build_streaming_group(saver: &AutoSaver) -> (adw::PreferencesGroup, adw::ExpanderRow) {
    let group = adw::PreferencesGroup::new();
    let cfg = saver.config();
    let is_dg = streaming_prov_idx(&cfg.streaming_provider) == 0;

    // The enable switch's handler lives in `pages::coupling`.
    let expander = adw::ExpanderRow::builder()
        .title("Real-Time Streaming")
        .subtitle("WebSocket-based transcription with ~300 ms latency. Requires direct injection.")
        .show_enable_switch(true)
        .enable_expansion(cfg.streaming_enabled)
        .build();

    let provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Deepgram", "AssemblyAI"]))
        .build();
    provider_row.set_selected(streaming_prov_idx(&cfg.streaming_provider));
    let dg_key_row = adw::PasswordEntryRow::builder()
        .title("Deepgram API Key")
        .text(&cfg.deepgram_api_key)
        .visible(is_dg)
        .build();
    let dg_model_row = adw::EntryRow::builder()
        .title("Deepgram Model")
        .text(&cfg.deepgram_model)
        .visible(is_dg)
        .build();
    let aai_key_row = adw::PasswordEntryRow::builder()
        .title("AssemblyAI API Key")
        .text(&cfg.assemblyai_api_key)
        .visible(!is_dg)
        .build();
    drop(cfg);

    provider_row.connect_selected_notify({
        let dg_key = dg_key_row.clone();
        let dg_model = dg_model_row.clone();
        let aai_key = aai_key_row.clone();
        let saver = saver.clone();
        move |row| {
            let is_dg = row.selected() == 0;
            dg_key.set_visible(is_dg);
            dg_model.set_visible(is_dg);
            aai_key.set_visible(!is_dg);
            let p = if is_dg { "deepgram" } else { "assemblyai" };
            saver.update(move |cfg| cfg.streaming_provider = p.to_string());
        }
    });

    dg_key_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| cfg.deepgram_api_key = t.clone());
        }
    });
    dg_model_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| cfg.deepgram_model = t.clone());
        }
    });
    aai_key_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| cfg.assemblyai_api_key = t.clone());
        }
    });

    expander.add_row(&provider_row);
    expander.add_row(&dg_key_row);
    expander.add_row(&dg_model_row);
    expander.add_row(&aai_key_row);
    group.add(&expander);
    (group, expander)
}

// ── History ──────────────────────────────────────────────────────────────────

fn build_history_group(saver: &AutoSaver) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title("History").build();

    let save_row = adw::SwitchRow::builder()
        .title("Save transcription history")
        .subtitle("Keep results and WAV recordings on disk")
        .active(saver.config().save_history)
        .build();

    let limit_row = adw::SpinRow::builder()
        .title("Entries to keep")
        .subtitle("Oldest entries and their recordings are deleted past this; 0 for unlimited")
        .adjustment(&gtk4::Adjustment::new(
            saver.config().history_max_entries as f64,
            0.0,
            100_000.0,
            50.0,
            500.0,
            0.0,
        ))
        .sensitive(saver.config().save_history)
        .build();

    save_row.connect_active_notify({
        let saver = saver.clone();
        let limit_row = limit_row.clone();
        move |row| {
            let v = row.is_active();
            limit_row.set_sensitive(v);
            saver.update(move |cfg| cfg.save_history = v);
        }
    });
    limit_row.connect_value_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.value() as u32;
            saver.update(move |cfg| cfg.history_max_entries = v);
        }
    });

    group.add(&save_row);
    group.add(&limit_row);
    group
}

// ── Audio ────────────────────────────────────────────────────────────────────

fn build_audio_group(saver: &AutoSaver) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title("Audio").build();
    let rate_row = adw::SpinRow::builder()
        .title("Sample Rate (Hz)")
        .subtitle("16 000 Hz recommended for Whisper")
        .adjustment(&gtk4::Adjustment::new(
            saver.config().sample_rate as f64,
            8000.0,
            48000.0,
            1000.0,
            8000.0,
            0.0,
        ))
        .build();
    rate_row.connect_value_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.value() as u32;
            saver.update(move |cfg| cfg.sample_rate = v);
        }
    });
    group.add(&rate_row);
    group
}

// ── Overlay (wlroots only) ───────────────────────────────────────────────────

fn build_overlay_group(saver: &AutoSaver) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Overlay")
        .description("For the Hyprland / Sway / KDE overlay")
        .build();

    let current = KEYBOARD_MODES
        .iter()
        .position(|m| *m == saver.config().overlay_keyboard_mode)
        .unwrap_or(1) as u32;
    let mode_row = adw::ComboRow::builder()
        .title("Keyboard focus")
        .subtitle(
            "“None” keeps focus on your window — use it if Esc-to-cancel is bound in \
             the compositor. Applies after the overlay restarts.",
        )
        .model(&gtk4::StringList::new(&["None", "On demand", "Exclusive"]))
        .build();
    mode_row.set_selected(current);
    mode_row.connect_selected_notify({
        let saver = saver.clone();
        move |row| {
            let m = KEYBOARD_MODES.get(row.selected() as usize).copied().unwrap_or("ondemand");
            saver.update(move |cfg| cfg.overlay_keyboard_mode = m.to_string());
        }
    });

    let gain_row = adw::SpinRow::builder()
        .title("Level meter gain")
        .subtitle("Lower it if the meter sits at maximum")
        .digits(1)
        .adjustment(&gtk4::Adjustment::new(
            saver.config().audio_level_gain,
            0.1,
            200.0,
            0.5,
            5.0,
            0.0,
        ))
        .build();
    gain_row.connect_value_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.value();
            saver.update(move |cfg| cfg.audio_level_gain = v);
        }
    });

    let floor_row = adw::SpinRow::builder()
        .title("Level meter noise floor")
        .subtitle("Raise it if background noise moves the meter")
        .digits(3)
        .adjustment(&gtk4::Adjustment::new(
            saver.config().audio_level_floor,
            0.0,
            1.0,
            0.001,
            0.01,
            0.0,
        ))
        .build();
    floor_row.connect_value_notify({
        let saver = saver.clone();
        move |row| {
            let v = row.value();
            saver.update(move |cfg| cfg.audio_level_floor = v);
        }
    });

    group.add(&mode_row);
    group.add(&gain_row);
    group.add(&floor_row);
    group
}
