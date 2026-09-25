//! Real-time streaming transcription via WebSocket.
//!
//! Supports Deepgram and AssemblyAI. Audio is forwarded as raw 16-bit LE PCM
//! chunks. The WebSocket task runs autonomously; callers interact through the
//! audio channel they pass in and the `StreamingSession` they get back.

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

/// How long to wait for the provider's final results after end-of-stream.
/// Both providers normally answer in well under a second; this only bounds a
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
    let task = tokio::spawn(ws_loop(
        ws_stream,
        audio_rx,
        event_tx,
        Provider {
            name:          "Deepgram",
            close_message: r#"{"type":"CloseStream"}"#,
            min_chunk:     0,
            max_chunk:     usize::MAX,
            parse:         parse_deepgram_message,
        },
    ));
    Ok(StreamingSession { event_rx, task })
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

/// Open an AssemblyAI (v3 Universal Streaming) session fed from `audio_rx`.
pub async fn start_assemblyai_session(
    api_key: &str,
    sample_rate: u32,
    language: Option<&str>,
    audio_rx: AudioRx,
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

    let ws_stream = connect(req).await?;
    let (event_tx, event_rx) = mpsc::channel::<StreamEvent>(64);
    let task = tokio::spawn(ws_loop(
        ws_stream,
        audio_rx,
        event_tx,
        Provider {
            name:          "AssemblyAI",
            close_message: r#"{"type":"Terminate"}"#,
            // v3 rejects binary frames outside 50–1000 ms of audio and closes
            // the session, while our forwarder ticks every ~33 ms.
            min_chunk:     sample_rate as usize / 20,
            max_chunk:     sample_rate as usize,
            parse:         parse_assemblyai_message,
        },
    ));
    Ok(StreamingSession { event_rx, task })
}

/// Parse a v3 Universal Streaming message.
///
/// With `format_turns=true` each finished turn arrives twice: first with
/// `end_of_turn: true` but unformatted, then again with `turn_is_formatted:
/// true`. Only the formatted copy is taken, otherwise every phrase would be
/// typed twice. Partial turns (`end_of_turn: false`) are ignored because the
/// injector can only append, never revise.
fn parse_assemblyai_message(text: &str) -> Option<StreamEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    match v.get("type")?.as_str()? {
        "Turn" => {
            let finished = v["end_of_turn"].as_bool() == Some(true)
                && v["turn_is_formatted"].as_bool() == Some(true);
            let transcript = v["transcript"].as_str()?.trim();
            (finished && !transcript.is_empty())
                .then(|| StreamEvent::Final(transcript.to_string()))
        }
        "Termination" => Some(StreamEvent::Done),
        _ => v["error"].as_str().map(|e| StreamEvent::Error(e.to_string())),
    }
}

// ── Shared WebSocket loop ─────────────────────────────────────────────────────

struct Provider {
    name:          &'static str,
    /// Text frame that asks the server to flush and close.
    close_message: &'static str,
    /// Samples per binary frame: buffer until at least `min_chunk`, split
    /// above `max_chunk`.
    min_chunk:     usize,
    max_chunk:     usize,
    parse:         fn(&str) -> Option<StreamEvent>,
}

async fn ws_loop(
    ws: WsConn,
    mut audio_rx: AudioRx,
    event_tx: mpsc::Sender<StreamEvent>,
    p: Provider,
) {
    let (mut sink, mut stream) = ws.split();
    let mut pending: Vec<i16> = Vec::new();

    loop {
        tokio::select! {
            chunk = audio_rx.recv() => {
                let end = chunk.is_none();
                if let Some(samples) = chunk {
                    pending.extend_from_slice(&samples);
                } else if !pending.is_empty() {
                    // Pad the tail rather than drop it: it holds the last
                    // syllable, and a short frame would be rejected.
                    pending.resize(pending.len().max(p.min_chunk), 0);
                }
                while !pending.is_empty() && (end || pending.len() >= p.min_chunk) {
                    let n = pending.len().min(p.max_chunk);
                    let bytes = pcm_to_bytes(&pending[..n]);
                    pending.drain(..n);
                    if let Err(e) = sink.send(Message::Binary(bytes.into())).await {
                        tracing::warn!("{}: send error: {e}", p.name);
                        let _ = event_tx.send(StreamEvent::Error(e.to_string())).await;
                        return;
                    }
                }
                if end {
                    tracing::debug!("{}: audio channel closed, requesting final results", p.name);
                    if let Err(e) = sink.send(Message::Text(p.close_message.into())).await {
                        tracing::warn!("{}: failed to send close message: {e}", p.name);
                    }
                    drain_ws(stream, event_tx, p).await;
                    return;
                }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let Some(ev) = (p.parse)(&text) else { continue };
                        let done = matches!(ev, StreamEvent::Done);
                        if event_tx.send(ev).await.is_err() || done { return; }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        tracing::debug!("{}: WebSocket closed by server: {frame:?}", p.name);
                        let _ = event_tx.send(close_event(frame)).await;
                        return;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("{}: WebSocket error: {e}", p.name);
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

/// A server-initiated close mid-recording is how both providers report
/// errors (bad key, unsupported params, malformed audio); only a normal close
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
async fn drain_ws(mut stream: WsStream, event_tx: mpsc::Sender<StreamEvent>, p: Provider) {
    let drain = async {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let Some(ev) = (p.parse)(&text) else { continue };
                    // Done is sent once, below, whichever way the drain ends.
                    if matches!(ev, StreamEvent::Done) || event_tx.send(ev).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    };
    if tokio::time::timeout(DRAIN_TIMEOUT, drain).await.is_err() {
        tracing::warn!("{}: no close after {}s, finishing with what we have", p.name, DRAIN_TIMEOUT.as_secs());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn final_text(ev: Option<StreamEvent>) -> Option<String> {
        match ev {
            Some(StreamEvent::Final(t)) => Some(t),
            _ => None,
        }
    }

    #[test]
    fn assemblyai_takes_only_formatted_end_of_turn() {
        let partial = r#"{"type":"Turn","transcript":"hello wor","end_of_turn":false,"turn_is_formatted":false}"#;
        let raw_end = r#"{"type":"Turn","transcript":"hello world","end_of_turn":true,"turn_is_formatted":false}"#;
        let formatted = r#"{"type":"Turn","transcript":"Hello world.","end_of_turn":true,"turn_is_formatted":true}"#;
        assert!(parse_assemblyai_message(partial).is_none());
        assert!(parse_assemblyai_message(raw_end).is_none());
        assert_eq!(final_text(parse_assemblyai_message(formatted)).as_deref(), Some("Hello world."));
        assert!(matches!(
            parse_assemblyai_message(r#"{"type":"Termination","audio_duration_seconds":3}"#),
            Some(StreamEvent::Done)
        ));
        assert!(parse_assemblyai_message(r#"{"type":"Begin","id":"x"}"#).is_none());
    }

    #[test]
    fn deepgram_takes_final_results() {
        let msg = r#"{"type":"Results","is_final":true,"channel":{"alternatives":[{"transcript":"hi there"}]}}"#;
        assert_eq!(final_text(parse_deepgram_message(msg)).as_deref(), Some("hi there"));
    }
}
