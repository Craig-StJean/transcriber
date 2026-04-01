use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Active provider: "groq" | "cohere" | "custom"
    #[serde(default = "default_provider")]
    pub provider: String,

    // ── Groq ─────────────────────────────────────────────────────────────────
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default = "default_groq_model")]
    pub groq_model: String,

    // ── Cohere ────────────────────────────────────────────────────────────────
    #[serde(default)]
    pub cohere_api_key: String,
    #[serde(default = "default_cohere_model")]
    pub cohere_model: String,

    // ── Custom endpoint ───────────────────────────────────────────────────────
    #[serde(default)]
    pub custom_api_url: String,
    #[serde(default)]
    pub custom_api_key: String,
    #[serde(default)]
    pub custom_model: String,

    // ── Shared ────────────────────────────────────────────────────────────────
    /// BCP-47 language code hint, e.g. Some("en"). None lets the API auto-detect.
    /// Note: Cohere requires a language; if None it defaults to "en".
    pub language: Option<String>,
    /// Microphone sample rate in Hz (16000 recommended for Whisper).
    pub sample_rate: u32,
    /// Whether to save transcription history (WAV recordings + results) to disk.
    #[serde(default = "default_true")]
    pub save_history: bool,
    /// Automatically stop recording after ~1.5 s of silence (Voice Activity Detection).
    #[serde(default)]
    pub vad_enabled: bool,
    /// Type text directly into the focused window instead of copying to clipboard.
    #[serde(default)]
    pub direct_injection: bool,
    /// Play brief audio cues when recording starts and transcription succeeds.
    #[serde(default)]
    pub audio_cues_enabled: bool,

    // ── Real-time streaming ───────────────────────────────────────────────────
    /// Stream audio over WebSocket for real-time transcription (~300 ms latency).
    /// Requires direct_injection to be enabled. Groq/Cohere do not support streaming.
    #[serde(default)]
    pub streaming_enabled: bool,
    /// Streaming provider: "deepgram" | "assemblyai"
    #[serde(default = "default_streaming_provider")]
    pub streaming_provider: String,
    /// Deepgram API key (from console.deepgram.com).
    #[serde(default)]
    pub deepgram_api_key: String,
    /// Deepgram model name (e.g. "nova-3").
    #[serde(default = "default_deepgram_model")]
    pub deepgram_model: String,
    /// AssemblyAI API key (from assemblyai.com).
    #[serde(default)]
    pub assemblyai_api_key: String,
}

fn default_provider()           -> String { "groq".into() }
fn default_groq_model()         -> String { "whisper-large-v3-turbo".into() }
fn default_cohere_model()       -> String { "cohere-transcribe-03-2026".into() }
fn default_streaming_provider() -> String { "deepgram".into() }
fn default_deepgram_model()     -> String { "nova-3".into() }
fn default_true()               -> bool   { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider:       default_provider(),
            groq_api_key:   String::new(),
            groq_model:     default_groq_model(),
            cohere_api_key: String::new(),
            cohere_model:   default_cohere_model(),
            custom_api_url: String::new(),
            custom_api_key: String::new(),
            custom_model:   String::new(),
            language:       Some("en".into()),
            sample_rate:    16000,
            save_history:       true,
            vad_enabled:        false,
            direct_injection:   false,
            audio_cues_enabled: false,
            streaming_enabled:    false,
            streaming_provider:   default_streaming_provider(),
            deepgram_api_key:     String::new(),
            deepgram_model:       default_deepgram_model(),
            assemblyai_api_key:   String::new(),
        }
    }
}

impl AppConfig {
    /// URL of the active transcription endpoint.
    pub fn active_url(&self) -> &str {
        match self.provider.as_str() {
            "groq"   => "https://api.groq.com/openai/v1/audio/transcriptions",
            "cohere" => "https://api.cohere.com/v2/audio/transcriptions",
            _        => &self.custom_api_url,
        }
    }

    /// API key for the active provider.
    pub fn active_key(&self) -> &str {
        match self.provider.as_str() {
            "groq"   => &self.groq_api_key,
            "cohere" => &self.cohere_api_key,
            _        => &self.custom_api_key,
        }
    }

    /// Model name for the active provider.
    pub fn active_model(&self) -> &str {
        match self.provider.as_str() {
            "groq"   => &self.groq_model,
            "cohere" => &self.cohere_model,
            _        => &self.custom_model,
        }
    }

    /// API key for the active streaming provider.
    pub fn active_streaming_key(&self) -> &str {
        match self.streaming_provider.as_str() {
            "assemblyai" => &self.assemblyai_api_key,
            _            => &self.deepgram_api_key,
        }
    }

    /// Language for the active provider.
    /// Cohere requires a language field; falls back to "en" if not configured.
    pub fn active_language(&self) -> Option<&str> {
        let lang = self.language.as_deref();
        if self.provider == "cohere" {
            Some(lang.unwrap_or("en"))
        } else {
            lang
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("could not locate config directory")
        .join("voice-transcriber")
        .join("config.json")
}

pub fn load() -> Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        let cfg = AppConfig::default();
        save(&cfg)?;
        return Ok(cfg);
    }
    let text = std::fs::read_to_string(&path)?;

    // Detect old single-provider config format and migrate it.
    let json: serde_json::Value = serde_json::from_str(&text)?;
    if json.get("provider").is_none() {
        let old_key   = json["api_key"].as_str().unwrap_or("").to_string();
        let old_url   = json["api_url"].as_str().unwrap_or("").to_string();
        let old_model = json["model"].as_str().unwrap_or("").to_string();

        let mut cfg = AppConfig::default();
        cfg.language    = json["language"].as_str().map(|s| s.to_string());
        cfg.sample_rate = json["sample_rate"].as_u64().unwrap_or(16000) as u32;

        if old_url.contains("groq.com") || old_url.is_empty() {
            cfg.provider     = "groq".into();
            cfg.groq_api_key = old_key;
            if !old_model.is_empty() { cfg.groq_model = old_model; }
        } else if old_url.contains("cohere.com") {
            cfg.provider        = "cohere".into();
            cfg.cohere_api_key  = old_key;
            if !old_model.is_empty() { cfg.cohere_model = old_model; }
        } else {
            cfg.provider        = "custom".into();
            cfg.custom_api_url  = old_url;
            cfg.custom_api_key  = old_key;
            cfg.custom_model    = old_model;
        }

        save(&cfg)?;
        return Ok(cfg);
    }

    Ok(serde_json::from_str(&text)?)
}

pub fn save(cfg: &AppConfig) -> Result<()> {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}
