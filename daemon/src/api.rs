use anyhow::{anyhow, Result};
use reqwest::multipart;

/// POST `wav_bytes` to an OpenAI-compatible `/audio/transcriptions` endpoint
/// and return the transcribed text.
pub async fn transcribe(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    language: Option<&str>,
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

    let response = client
        .post(api_url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    response["text"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow!("missing 'text' field in API response: {response}"))
}
