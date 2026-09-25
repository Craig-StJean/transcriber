use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::multipart;

/// A non-2xx response, with enough of the body kept to say *why*.
///
/// `error_for_status()` discards the body, and the body is where providers put
/// the only useful part ("file too large", "invalid model", quota details).
#[derive(Debug)]
pub struct ApiError {
    pub status:      reqwest::StatusCode,
    /// Response body, truncated to `BODY_LIMIT` characters.
    pub body:        String,
    /// Parsed `Retry-After` header (delta-seconds form only), if present.
    pub retry_after: Option<Duration>,
}

const BODY_LIMIT: usize = 500;

impl ApiError {
    /// Worth retrying: rate limits and server-side failures. Everything else
    /// (bad key, bad model, file too large) will fail identically next time.
    pub fn is_retryable(&self) -> bool {
        self.status == reqwest::StatusCode::TOO_MANY_REQUESTS || self.status.is_server_error()
    }

    /// Short form for the UI, e.g. "413 file too large". Prefers the
    /// provider's own message (OpenAI-style `error.message`, or a top-level
    /// `message`) over the generic reason phrase.
    pub fn short(&self) -> String {
        let detail = serde_json::from_str::<serde_json::Value>(&self.body)
            .ok()
            .and_then(|v| {
                v["error"]["message"].as_str()
                    .or_else(|| v["error"].as_str())
                    .or_else(|| v["message"].as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| self.status.canonical_reason().unwrap_or("error").to_lowercase());
        format!("{} {}", self.status.as_u16(), truncate(&detail, 160))
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}: {}", self.status, self.body)
    }
}

impl std::error::Error for ApiError {}

/// Turn a non-2xx response into an `ApiError`, passing successes through.
async fn check_status(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(Duration::from_secs_f64);
    let body = response.text().await.unwrap_or_default();
    Err(ApiError { status, body: truncate(body.trim(), BODY_LIMIT), retry_after }.into())
}

fn truncate(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None         => s.to_string(),
    }
}

/// POST `wav_bytes` to an OpenAI-compatible `/audio/transcriptions` endpoint
/// and return the transcribed text.
///
/// `prompt` is Whisper's spelling/style hint (see `AppConfig::active_prompt`).
/// It is capped at 224 tokens upstream and silently truncated beyond that, so
/// callers should budget for the limit rather than relying on the API to say so.
pub async fn transcribe(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    language: Option<&str>,
    prompt: Option<&str>,
    wav_bytes: Vec<u8>,
) -> Result<String> {
    let file_part = multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let mut form = multipart::Form::new()
        .text("model", model.to_string())
        .part("file", file_part);

    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }

    if let Some(p) = prompt.filter(|p| !p.is_empty()) {
        form = form.text("prompt", p.to_string());
    }

    let response = client
        .post(api_url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;
    let response = check_status(response)
        .await?
        .json::<serde_json::Value>()
        .await?;

    response["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow!("missing 'text' field in API response: {response}"))
}

/// Run `transcript` through an OpenAI-compatible chat-completions endpoint with
/// `system_prompt` as the system message, returning the assistant's reply.
pub async fn postprocess(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    transcript: &str,
) -> Result<String> {
    if api_url.is_empty() {
        return Err(anyhow!("post-processing endpoint URL is empty"));
    }
    if api_key.is_empty() {
        return Err(anyhow!("post-processing API key is empty"));
    }

    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user",   "content": transcript    },
        ],
    });

    let response = client
        .post(api_url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    let response = check_status(response)
        .await?
        .json::<serde_json::Value>()
        .await?;

    response["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow!("missing 'choices[0].message.content' in response: {response}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(status: u16, body: &str) -> ApiError {
        ApiError {
            status: reqwest::StatusCode::from_u16(status).unwrap(),
            body: body.into(),
            retry_after: None,
        }
    }

    #[test]
    fn short_prefers_provider_message() {
        let e = err(413, r#"{"error":{"message":"file too large","type":"invalid_request_error"}}"#);
        assert_eq!(e.short(), "413 file too large");
        assert_eq!(err(502, "<html>").short(), "502 bad gateway");
    }

    #[test]
    fn retryable_statuses() {
        assert!(err(429, "").is_retryable());
        assert!(err(503, "").is_retryable());
        assert!(!err(401, "").is_retryable());
        assert!(!err(413, "").is_retryable());
    }

    #[test]
    fn truncate_is_char_safe() {
        assert_eq!(truncate("héllo", 2), "hé…");
        assert_eq!(truncate("hi", 5), "hi");
    }
}
