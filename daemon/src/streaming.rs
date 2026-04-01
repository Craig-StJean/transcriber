//! Real-time streaming transcription via WebSocket.
//!
//! Supports Deepgram and AssemblyAI. Audio is forwarded as raw 16-bit LE PCM
//! chunks (~100 ms each). The WebSocket task runs autonomously; callers interact
//! through the `StreamingSession` channels.

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{http::Request, Message},
};

// ── Public types ──────────────────────────────────────────────────────────────

/// Events emitted by a live streaming session.
pub enum StreamEvent {
    /// A finalized phrase — append to the accumulated transcript.
    Final(String),
    /// The session closed cleanly; no more events will be sent.
    Done,
    /// The session terminated with an error.
    Error(String),
}

/// Handle to a live streaming session.
pub struct StreamingSession {
    /// Send raw i16 PCM chunks here. Drop this sender to signal end-of-stream.
    pub audio_tx: mpsc::Sender<Vec<i16>>,
    /// Receive transcript events from this receiver.
    pub event_rx: mpsc::Receiver<StreamEvent>,
}

// Type alias for the stream half of a split WebSocket connection.
type WsStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
>;

// ── Deepgram ──────────────────────────────────────────────────────────────────

/// Open a Deepgram streaming session.
///
/// Returns a `StreamingSession` whose `audio_tx` accepts raw i16 PCM chunks.
/// Drop `audio_tx` to signal end-of-stream.
pub async fn start_deepgram_session(
    api_key: &str,
    model: &str,
    sample_rate: u32,
    language: Option<&str>,
) -> Result<StreamingSession> {
    let lang_param = language
        .map(|l| format!("&language={l}"))
        .unwrap_or_default();

    let url = format!(
        "wss://api.deepgram.com/v1/listen\
         ?model={model}\
         &encoding=linear16\
         &sample_rate={sample_rate}\
         &channels=1\
         &interim_results=true\
         &endpointing=300\
         {lang_param}"
    );

    let req = Request::builder()
        .uri(&url)
        .header("Authorization", format!("Token {api_key}"))
        .body(())
        .context("failed to build Deepgram WebSocket request")?;

    let (ws_stream, _) = connect_async_tls_with_config(req, None, false, None)
        .await
        .context("failed to connect to Deepgram WebSocket")?;

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(32);
    let (event_tx, event_rx) = mpsc::channel::<StreamEvent>(64);

    tokio::spawn(deepgram_ws_loop(ws_stream, audio_rx, event_tx));

    Ok(StreamingSession { audio_tx, event_rx })
}

async fn deepgram_ws_loop(
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    event_tx: mpsc::Sender<StreamEvent>,
) {
    let (mut sink, mut stream) = ws.split();

    loop {
        tokio::select! {
            chunk = audio_rx.recv() => {
                match chunk {
                    Some(samples) => {
                        let bytes = pcm_to_bytes(&samples);
                        if let Err(e) = sink.send(Message::Binary(bytes.into())).await {
                            tracing::warn!("Deepgram: send error: {e}");
                            let _ = event_tx.send(StreamEvent::Error(e.to_string())).await;
                            return;
                        }
                    }
                    None => {
                        // audio_tx dropped — signal end-of-stream
                        tracing::debug!("Deepgram: audio channel closed, sending CloseStream");
                        let close_msg = r#"{"type":"CloseStream"}"#;
                        if let Err(e) = sink.send(Message::Text(close_msg.into())).await {
                            tracing::warn!("Deepgram: failed to send CloseStream: {e}");
                        }
                        drain_ws(stream, event_tx, parse_deepgram_message).await;
                        return;
                    }
                }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(ev) = parse_deepgram_message(&text) {
                            if event_tx.send(ev).await.is_err() { return; }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("Deepgram: WebSocket closed by server");
                        let _ = event_tx.send(StreamEvent::Done).await;
                        return;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("Deepgram: WebSocket error: {e}");
                        let _ = event_tx.send(StreamEvent::Error(e.to_string())).await;
                        return;
                    }
                    None => {
                        let _ = event_tx.send(StreamEvent::Done).await;
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn parse_deepgram_message(text: &str) -> Option<StreamEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("type")?.as_str()? == "Results" && v["is_final"].as_bool() == Some(true) {
        let transcript = v["channel"]["alternatives"][0]["transcript"]
            .as_str()?
            .to_string();
        if !transcript.is_empty() {
            return Some(StreamEvent::Final(transcript));
        }
    }
    None
}

// ── AssemblyAI ────────────────────────────────────────────────────────────────

/// Open an AssemblyAI streaming session.
pub async fn start_assemblyai_session(
    api_key: &str,
    sample_rate: u32,
    language: Option<&str>,
) -> Result<StreamingSession> {
    let lang_param = language
        .map(|l| format!("&language_code={l}"))
        .unwrap_or_default();

    let url = format!(
        "wss://streaming.assemblyai.com/v3/ws\
         ?sample_rate={sample_rate}\
         &encoding=pcm_s16le\
         &format_turns=true\
         {lang_param}"
    );

    let req = Request::builder()
        .uri(&url)
        .header("Authorization", api_key)
        .body(())
        .context("failed to build AssemblyAI WebSocket request")?;

    let (ws_stream, _) = connect_async_tls_with_config(req, None, false, None)
        .await
        .context("failed to connect to AssemblyAI WebSocket")?;

    let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(32);
    let (event_tx, event_rx) = mpsc::channel::<StreamEvent>(64);

    tokio::spawn(assemblyai_ws_loop(ws_stream, audio_rx, event_tx));

    Ok(StreamingSession { audio_tx, event_rx })
}

async fn assemblyai_ws_loop(
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    event_tx: mpsc::Sender<StreamEvent>,
) {
    let (mut sink, mut stream) = ws.split();

    loop {
        tokio::select! {
            chunk = audio_rx.recv() => {
                match chunk {
                    Some(samples) => {
                        let bytes = pcm_to_bytes(&samples);
                        if let Err(e) = sink.send(Message::Binary(bytes.into())).await {
                            tracing::warn!("AssemblyAI: send error: {e}");
                            let _ = event_tx.send(StreamEvent::Error(e.to_string())).await;
                            return;
                        }
                    }
                    None => {
                        tracing::debug!("AssemblyAI: audio channel closed, sending Terminate");
                        let terminate = r#"{"type":"Terminate"}"#;
                        if let Err(e) = sink.send(Message::Text(terminate.into())).await {
                            tracing::warn!("AssemblyAI: failed to send Terminate: {e}");
                        }
                        drain_ws(stream, event_tx, parse_assemblyai_message).await;
                        return;
                    }
                }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(ev) = parse_assemblyai_message(&text) {
                            if event_tx.send(ev).await.is_err() { return; }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("AssemblyAI: WebSocket closed by server");
                        let _ = event_tx.send(StreamEvent::Done).await;
                        return;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("AssemblyAI: WebSocket error: {e}");
                        let _ = event_tx.send(StreamEvent::Error(e.to_string())).await;
                        return;
                    }
                    None => {
                        let _ = event_tx.send(StreamEvent::Done).await;
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn parse_assemblyai_message(text: &str) -> Option<StreamEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("type")?.as_str()? == "transcript" {
        let t = &v["transcript"];
        if t["type"].as_str()? == "final" {
            let transcript = t["text"].as_str()?.to_string();
            if !transcript.is_empty() {
                return Some(StreamEvent::Final(transcript));
            }
        }
    }
    None
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Drain remaining messages from a WebSocket stream after end-of-stream signal.
async fn drain_ws(
    mut stream: WsStream,
    event_tx: mpsc::Sender<StreamEvent>,
    parse: fn(&str) -> Option<StreamEvent>,
) {
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Some(ev) = parse(&text) {
                    if event_tx.send(ev).await.is_err() {
                        return;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
    let _ = event_tx.send(StreamEvent::Done).await;
}

fn pcm_to_bytes(samples: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Convert f32 samples to raw i16 LE PCM (no WAV header).
/// Used by `dbus.rs` to forward audio chunks to the WebSocket.
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect()
}
