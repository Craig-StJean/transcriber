//! Connection tests for the settings app's "Test connections" button.
//!
//! Each configured provider gets one cheap authenticated GET — a model list or
//! a project list — rather than a real transcription, so a test costs nothing
//! and needs no audio. Failures are reported per provider, never as a DBus
//! error: a bad Deepgram key shouldn't hide that Groq is fine.

use std::time::Duration;

use anyhow::Result;
use serde::Serialize;

use crate::api::{self, ApiError};
use common::config::AppConfig;

/// Per-check budget. The daemon's shared client allows 120 s for uploads,
/// which is far too long for someone watching a spinner.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

const GROQ_MODELS_URL:       &str = "https://api.groq.com/openai/v1/models";
const DEEPGRAM_PROJECTS_URL: &str = "https://api.deepgram.com/v1/projects";

#[derive(Debug, Serialize)]
pub struct Check {
    pub name:   String,
    pub ok:     bool,
    pub detail: String,
}

impl Check {
    fn new(name: String, result: Result<String, String>) -> Self {
        match result {
            Ok(detail)  => Self { name, ok: true,  detail },
            Err(detail) => Self { name, ok: false, detail },
        }
    }
}

// ── Labels and preconditions (shared with dbus.rs) ────────────────────────────

pub fn provider_label(cfg: &AppConfig) -> &'static str {
    match cfg.provider.as_str() {
        "groq" => "Groq",
        _      => "Custom endpoint",
    }
}

pub fn postprocess_label(cfg: &AppConfig) -> &'static str {
    match cfg.postprocess_provider.as_str() {
        "groq"   => "Groq",
        "gemini" => "Gemini",
        _        => "Custom endpoint",
    }
}

/// Why post-processing can't run with this config, if it can't. Independent
/// of `postprocess_enabled`: re-polishing a history entry is allowed with the
/// live pass switched off, as long as a provider is set up.
pub fn postprocess_problem(cfg: &AppConfig) -> Option<String> {
    let label = postprocess_label(cfg);
    if cfg.active_postprocess_url().is_empty() {
        Some(format!("no {label} URL configured"))
    } else if cfg.active_postprocess_key().is_empty() {
        Some(format!("no {label} API key configured"))
    } else {
        None
    }
}

// ── Checks ────────────────────────────────────────────────────────────────────

/// Test every provider the config would actually use, concurrently.
/// Streaming and post-processing are skipped unless enabled.
pub async fn test_all(client: &reqwest::Client, cfg: &AppConfig) -> Vec<Check> {
    let transcription = async {
        Check::new(
            format!("Transcription ({})", provider_label(cfg)),
            check_transcription(client, cfg).await,
        )
    };
    let streaming = async {
        if !cfg.streaming_enabled {
            return None;
        }
        Some(Check::new("Streaming (Deepgram)".into(), check_deepgram(client, cfg).await))
    };
    let postprocess = async {
        if !cfg.postprocess_enabled {
            return None;
        }
        Some(Check::new(
            format!("Post-processing ({})", postprocess_label(cfg)),
            check_postprocess(client, cfg).await,
        ))
    };

    let (t, s, p) = tokio::join!(transcription, streaming, postprocess);
    std::iter::once(t).chain(s).chain(p).collect()
}

async fn check_transcription(client: &reqwest::Client, cfg: &AppConfig) -> Result<String, String> {
    if cfg.provider == "groq" {
        if cfg.groq_api_key.is_empty() {
            return Err("no Groq API key configured".into());
        }
        let list = get(client, GROQ_MODELS_URL, Auth::Bearer(&cfg.groq_api_key)).await.map_err(fmt_err)?;
        return require_model(&list, &cfg.groq_model);
    }

    if cfg.custom_api_url.is_empty() {
        return Err("no endpoint URL configured".into());
    }
    // Custom endpoints may be keyless (a local whisper server), so no key is
    // not an error here — the server will say so if it wants one.
    let url = models_url(&cfg.custom_api_url, "/audio/transcriptions");
    probe_custom(client, &url, &cfg.custom_api_key, &cfg.custom_model).await
}

async fn check_deepgram(client: &reqwest::Client, cfg: &AppConfig) -> Result<String, String> {
    if cfg.deepgram_api_key.is_empty() {
        return Err("no Deepgram API key configured".into());
    }
    get(client, DEEPGRAM_PROJECTS_URL, Auth::Token(&cfg.deepgram_api_key)).await.map_err(fmt_err)?;
    Ok(ok_detail(&cfg.deepgram_model))
}

async fn check_postprocess(client: &reqwest::Client, cfg: &AppConfig) -> Result<String, String> {
    if let Some(problem) = postprocess_problem(cfg) {
        return Err(problem);
    }
    let url   = models_url(cfg.active_postprocess_url(), "/chat/completions");
    let key   = cfg.active_postprocess_key();
    let model = cfg.active_postprocess_model();
    match cfg.postprocess_provider.as_str() {
        "groq" | "gemini" => {
            let list = get(client, &url, Auth::Bearer(key)).await.map_err(fmt_err)?;
            require_model(&list, model)
        }
        _ => probe_custom(client, &url, key, model).await,
    }
}

/// A user-supplied OpenAI-compatible server: any 2xx proves the URL and key,
/// and a 404 still proves the server is up — plenty of local servers
/// implement only the one endpoint we need and no model list.
async fn probe_custom(client: &reqwest::Client, url: &str, key: &str, model: &str) -> Result<String, String> {
    let auth = if key.is_empty() { Auth::None } else { Auth::Bearer(key) };
    match get(client, url, auth).await {
        Ok(_) => Ok(ok_detail(model)),
        Err(e) if e.downcast_ref::<ApiError>().is_some_and(|a| a.status == reqwest::StatusCode::NOT_FOUND) => {
            Ok("reachable (no /models endpoint)".into())
        }
        Err(e) => Err(fmt_err(e)),
    }
}

fn ok_detail(model: &str) -> String {
    if model.is_empty() { "OK".into() } else { format!("OK — {model}") }
}

fn require_model(list: &serde_json::Value, model: &str) -> Result<String, String> {
    if model_listed(list, model) {
        Ok(ok_detail(model))
    } else {
        Err(format!("model {model} not found"))
    }
}

fn fmt_err(e: anyhow::Error) -> String {
    tracing::warn!("connection test failed: {e}");
    api::error_detail(&e)
}

// ── HTTP ──────────────────────────────────────────────────────────────────────

enum Auth<'a> {
    None,
    Bearer(&'a str),
    /// Deepgram's scheme: `Authorization: Token <key>`.
    Token(&'a str),
}

async fn get(client: &reqwest::Client, url: &str, auth: Auth<'_>) -> Result<serde_json::Value> {
    let req = client.get(url).timeout(PROBE_TIMEOUT);
    let req = match auth {
        Auth::None      => req,
        Auth::Bearer(k) => req.bearer_auth(k),
        Auth::Token(k)  => req.header(reqwest::header::AUTHORIZATION, format!("Token {k}")),
    };
    let response = api::check_status(req.send().await?).await?;
    // A non-JSON 2xx still proves the credentials; it just lists no models.
    Ok(response.json().await.unwrap_or(serde_json::Value::Null))
}

// ── Pure helpers ──────────────────────────────────────────────────────────────

/// The OpenAI-style `/models` URL for an endpoint URL: strip `suffix` (the
/// operation path) and a trailing slash, then append `/models`. A URL that
/// doesn't end in `suffix` is treated as already being the API base.
fn models_url(endpoint: &str, suffix: &str) -> String {
    let base = endpoint.trim().trim_end_matches('/');
    let base = base.strip_suffix(suffix).unwrap_or(base).trim_end_matches('/');
    format!("{base}/models")
}

/// Whether `model` appears in an OpenAI-style `{"data":[{"id":…}]}` list.
/// Gemini's compatibility endpoint lists ids as "models/<name>" while the
/// chat endpoint accepts the bare name, so either form matches either form.
fn model_listed(list: &serde_json::Value, model: &str) -> bool {
    let bare = |s: &str| s.strip_prefix("models/").unwrap_or(s).to_string();
    let want = bare(model.trim());
    !want.is_empty()
        && list["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m["id"].as_str())
            .any(|id| bare(id) == want)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn models_url_replaces_operation_path() {
        assert_eq!(
            models_url("https://api.groq.com/openai/v1/chat/completions", "/chat/completions"),
            "https://api.groq.com/openai/v1/models"
        );
        assert_eq!(
            models_url(
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
                "/chat/completions"
            ),
            "https://generativelanguage.googleapis.com/v1beta/openai/models"
        );
        assert_eq!(
            models_url("http://localhost:8000/v1/audio/transcriptions/", "/audio/transcriptions"),
            "http://localhost:8000/v1/models"
        );
        assert_eq!(models_url("http://localhost:8000/v1", "/audio/transcriptions"), "http://localhost:8000/v1/models");
    }

    #[test]
    fn model_listed_accepts_gemini_prefix_either_way() {
        let gemini = json!({"data": [{"id": "models/gemini-3-flash-preview"}]});
        assert!(model_listed(&gemini, "gemini-3-flash-preview"));
        assert!(model_listed(&gemini, "models/gemini-3-flash-preview"));
        assert!(!model_listed(&gemini, "gemini-2.5-pro"));

        let groq = json!({"data": [{"id": "whisper-large-v3-turbo"}, {"id": "llama-3.3-70b-versatile"}]});
        assert!(model_listed(&groq, "whisper-large-v3-turbo"));
        assert!(model_listed(&groq, "models/whisper-large-v3-turbo"));
        assert!(!model_listed(&groq, "whisper-large-v3"));
        assert!(!model_listed(&groq, ""));
        assert!(!model_listed(&serde_json::Value::Null, "whisper-large-v3-turbo"));
    }
}
