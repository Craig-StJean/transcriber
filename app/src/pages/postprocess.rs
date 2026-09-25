use common::config;
use libadwaita as adw;
use adw::prelude::*;

use crate::saver::AutoSaver;
use crate::ui;

fn provider_idx(p: &str) -> u32 {
    match p { "gemini" => 1, "custom" => 2, _ => 0 }
}

fn provider_str(idx: u32) -> &'static str {
    match idx { 1 => "gemini", 2 => "custom", _ => "groq" }
}

/// Returns the page and its enable switch; the switch's handler lives in
/// `pages::coupling`, since flipping it affects streaming and vocabulary.
pub fn build(saver: &AutoSaver, toast: &adw::ToastOverlay) -> (adw::PreferencesPage, adw::SwitchRow) {
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
        .active(saver.config().postprocess_enabled)
        .build();
    enable_group.add(&enable_row);

    // ── Provider group ──────────────────────────────────────────────────────
    let provider_group = adw::PreferencesGroup::builder().title("LLM Provider").build();

    let cfg = saver.config();
    let provider_row = adw::ComboRow::builder()
        .title("Provider")
        .model(&gtk4::StringList::new(&["Groq", "Gemini", "Custom"]))
        .build();
    provider_row.set_selected(provider_idx(&cfg.postprocess_provider));
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

    // As on the Settings page: the rows already wrote their text to the
    // outgoing provider's fields, so switching only repoints them.
    provider_row.connect_selected_notify({
        let saver = saver.clone();
        let api_key_row = api_key_row.clone();
        let model_row = model_row.clone();
        let url_row = url_row.clone();
        move |row| {
            let provider = provider_str(row.selected());
            saver.update(move |cfg| cfg.postprocess_provider = provider.to_string());
            let (key, model) = {
                let c = saver.config();
                (c.active_postprocess_key().to_string(), c.active_postprocess_model().to_string())
            };
            api_key_row.set_text(&key);
            model_row.set_text(&model);
            url_row.set_visible(provider == "custom");
        }
    });

    api_key_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| match cfg.postprocess_provider.as_str() {
                "groq"   => cfg.postprocess_groq_api_key   = t.clone(),
                "gemini" => cfg.postprocess_gemini_api_key = t.clone(),
                _        => cfg.postprocess_custom_api_key = t.clone(),
            });
        }
    });

    model_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| match cfg.postprocess_provider.as_str() {
                "groq"   => cfg.postprocess_groq_model   = t.clone(),
                "gemini" => cfg.postprocess_gemini_model = t.clone(),
                _        => cfg.postprocess_custom_model = t.clone(),
            });
        }
    });

    url_row.connect_changed({
        let saver = saver.clone();
        move |row| {
            let t = row.text().to_string();
            saver.update(move |cfg| cfg.postprocess_custom_api_url = t.clone());
        }
    });

    provider_group.add(&provider_row);
    provider_group.add(&api_key_row);
    provider_group.add(&model_row);
    provider_group.add(&url_row);

    // ── Prompt group ────────────────────────────────────────────────────────
    let prompt_group = adw::PreferencesGroup::builder()
        .title("System Prompt")
        .description(
            "Sent to the LLM along with the raw transcript. Your vocabulary \
             (Settings → Vocabulary) is appended automatically as the list of \
             authoritative spellings — no need to repeat it here.",
        )
        .build();

    let prompt_buffer = gtk4::TextBuffer::new(None);
    prompt_buffer.set_text(&saver.config().postprocess_prompt);

    let prompt_view = gtk4::TextView::builder()
        .buffer(&prompt_buffer)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();
    prompt_view.update_property(&[gtk4::accessible::Property::Label("Post-processing system prompt")]);

    let prompt_scroll = gtk4::ScrolledWindow::builder()
        .height_request(160)
        .child(&prompt_view)
        .build();
    let prompt_frame = gtk4::Frame::builder().child(&prompt_scroll).build();
    prompt_frame.add_css_class("card");

    prompt_buffer.connect_changed({
        let saver = saver.clone();
        move |buf| {
            let t = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            saver.update(move |cfg| cfg.postprocess_prompt = t.clone());
        }
    });

    let reset_btn = gtk4::Button::builder()
        .label("Reset to Default")
        .halign(gtk4::Align::End)
        .build();
    reset_btn.connect_clicked({
        let prompt_buffer = prompt_buffer.clone();
        let toast = toast.clone();
        move |_| {
            ui::reset_buffer_with_undo(&toast, &prompt_buffer, config::DEFAULT_POSTPROCESS_PROMPT, "Prompt")
        }
    });

    let prompt_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    prompt_box.append(&prompt_frame);
    prompt_box.append(&reset_btn);
    prompt_group.add(&prompt_box);

    page.add(&enable_group);
    page.add(&provider_group);
    page.add(&prompt_group);
    (page, enable_row)
}
