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
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    response["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow!("missing 'choices[0].message.content' in response: {response}"))
}
