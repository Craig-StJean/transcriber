//! Real-time streaming transcription via WebSocket.
//!
//! Deepgram only. Audio is forwarded as raw 16-bit LE PCM chunks. The
//! WebSocket task runs autonomously; callers interact through the audio
//! channel they pass in and the `StreamingSession` they get back.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{http::Request, protocol::frame::coding::CloseCode, Message},
};

/// How long the WebSocket handshake may take before we give up. Without this
/// a black-holed connection would leave the daemon in Recording indefinitely.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait for Deepgram's final results after end-of-stream.
/// It normally answers in well under a second; this only bounds a
/// server that never closes the socket.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

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
    /// Receive transcript events from this receiver.
    pub event_rx: mpsc::Receiver<StreamEvent>,
    /// The WebSocket task. Abort it to cancel without flushing: unlike
    /// dropping the audio sender, that yields no further transcript.
    pub task: tokio::task::JoinHandle<()>,
}

/// Raw i16 PCM chunks for the session. Dropping every sender is the
/// end-of-stream signal. Unbounded because the producer is a real-time audio
/// loop that must never block or drop, and the total is already bounded by
/// the recording length cap.
pub type AudioRx = mpsc::UnboundedReceiver<Vec<i16>>;

type WsConn = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

// Type alias for the stream half of a split WebSocket connection.
type WsStream = futures_util::stream::SplitStream<WsConn>;

async fn connect(req: Request<()>) -> Result<WsConn> {
    let (ws, _) = tokio::time::timeout(CONNECT_TIMEOUT, connect_async_tls_with_config(req, None, false, None))
        .await
        .map_err(|_| anyhow!("connection timed out"))?
        .context("could not connect")?;
    Ok(ws)
}

// ── Deepgram ──────────────────────────────────────────────────────────────────

/// Open a Deepgram streaming session fed from `audio_rx`.
pub async fn start_deepgram_session(
    api_key: &str,
    model: &str,
    sample_rate: u32,
    language: Option<&str>,
    audio_rx: AudioRx,
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

    let ws_stream = connect(req).await?;
    let (event_tx, event_rx) = mpsc::channel::<StreamEvent>(64);
    let task = tokio::spawn(ws_loop(ws_stream, audio_rx, event_tx));
    Ok(StreamingSession { event_rx, task })
}

/// The finalized transcript in a Deepgram message, if it carries one.
/// Interim results are ignored because the injector can only append, never
/// revise.
fn parse_deepgram_message(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("type")?.as_str()? == "Results" && v["is_final"].as_bool() == Some(true) {
        let transcript = v["channel"]["alternatives"][0]["transcript"].as_str()?;
        if !transcript.is_empty() {
            return Some(transcript.to_string());
        }
    }
    None
}

/// Text frame that asks Deepgram to flush its final results and close.
const CLOSE_MESSAGE: &str = r#"{"type":"CloseStream"}"#;

// ── WebSocket loop ────────────────────────────────────────────────────────────

async fn ws_loop(ws: WsConn, mut audio_rx: AudioRx, event_tx: mpsc::Sender<StreamEvent>) {
    let (mut sink, mut stream) = ws.split();

    loop {
        tokio::select! {
            chunk = audio_rx.recv() => {
                let Some(samples) = chunk else {
                    tracing::debug!("Deepgram: audio channel closed, requesting final results");
                    if let Err(e) = sink.send(Message::Text(CLOSE_MESSAGE.into())).await {
                        tracing::warn!("Deepgram: failed to send close message: {e}");
                    }
                    drain_ws(stream, event_tx).await;
                    return;
                };
                if samples.is_empty() {
                    continue;
                }
                if let Err(e) = sink.send(Message::Binary(pcm_to_bytes(&samples).into())).await {
                    tracing::warn!("Deepgram: send error: {e}");
                    let _ = event_tx.send(StreamEvent::Error(e.to_string())).await;
                    return;
                }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let Some(phrase) = parse_deepgram_message(&text) else { continue };
                        if event_tx.send(StreamEvent::Final(phrase)).await.is_err() { return; }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        tracing::debug!("Deepgram: WebSocket closed by server: {frame:?}");
                        let _ = event_tx.send(close_event(frame)).await;
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

/// A server-initiated close mid-recording is how Deepgram reports errors (bad key, unsupported params, malformed audio); only a normal close
/// means "done".
fn close_event(frame: Option<tokio_tungstenite::tungstenite::protocol::CloseFrame>) -> StreamEvent {
    match frame {
        Some(f) if f.code != CloseCode::Normal => {
            let code: u16 = f.code.into();
            StreamEvent::Error(if f.reason.is_empty() {
                format!("connection closed ({code})")
            } else {
                format!("{} ({code})", f.reason)
            })
        }
        _ => StreamEvent::Done,
    }
}

/// Drain remaining messages from a WebSocket stream after end-of-stream signal.
async fn drain_ws(mut stream: WsStream, event_tx: mpsc::Sender<StreamEvent>) {
    let drain = async {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let Some(phrase) = parse_deepgram_message(&text) else { continue };
                    if event_tx.send(StreamEvent::Final(phrase)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    };
    if tokio::time::timeout(DRAIN_TIMEOUT, drain).await.is_err() {
        tracing::warn!("Deepgram: no close after {}s, finishing with what we have", DRAIN_TIMEOUT.as_secs());
    }
    // Done is sent once, here, whichever way the drain ended.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepgram_takes_final_results() {
        let msg = r#"{"type":"Results","is_final":true,"channel":{"alternatives":[{"transcript":"hi there"}]}}"#;
        assert_eq!(parse_deepgram_message(msg).as_deref(), Some("hi there"));
        let interim = r#"{"type":"Results","is_final":false,"channel":{"alternatives":[{"transcript":"hi"}]}}"#;
        assert!(parse_deepgram_message(interim).is_none());
    }
}
