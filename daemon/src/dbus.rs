use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tokio::sync::{mpsc, watch};
use zbus::{interface, names::InterfaceName, object_server::SignalEmitter, zvariant::Value, Connection};

use crate::api::ApiError;
use crate::audio::{self, PcmBuffer};
use crate::db::Database;
use crate::streaming::{self, StreamEvent};
use common::config::AppConfig;

// ── State machine ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonState {
    Idle,
    Recording,
    Transcribing,
    PostProcessing,
    Streaming,
    Done,
    Error(String),
}

impl DaemonState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle           => "Idle",
            Self::Recording      => "Recording",
            Self::Transcribing   => "Transcribing",
            Self::PostProcessing => "PostProcessing",
            Self::Streaming      => "Streaming",
            Self::Done           => "Done",
            Self::Error(_)       => "Error",
        }
    }

    fn is_busy(&self) -> bool {
        matches!(self, Self::Recording | Self::Transcribing | Self::PostProcessing | Self::Streaming)
    }

    /// States that end a session; entering one releases whatever capture
    /// resources the session still holds.
    fn is_terminal(&self) -> bool {
        matches!(self, Self::Idle | Self::Done | Self::Error(_))
    }
}

/// Recordings shorter than this are treated as accidental key presses.
const MIN_RECORDING_SECS: f32 = 0.3;

/// The daemon state plus the session it belongs to.
///
/// `generation` is bumped by every StartRecording and Cancel. Background work
/// (the batch pipeline, the streaming consumer, the VU/VAD loop) captures the
/// generation it was started under and only emits signals, writes history or
/// changes state while that is still current — otherwise a cancelled
/// transcription would still type its text, or a stale pipeline would
/// overwrite the state of the *next* recording.
struct StateCell {
    state:           DaemonState,
    generation:      u64,
    /// Whether the current session uses the streaming path.
    streaming:       bool,
    /// Whether the VU loop has seen speech-level audio in this session.
    speech_detected: bool,
}

/// Resources held only while the microphone is open.
#[derive(Default)]
struct Capture {
    /// Sender to stop the cpal capture thread.
    stop_tx:     Option<watch::Sender<bool>>,
    /// The live audio buffer.
    audio_buf:   Option<PcmBuffer>,
    /// The streaming WebSocket task, so Cancel can kill it outright.
    stream_task: Option<tokio::task::AbortHandle>,
}

impl Capture {
    /// Stop the microphone and release everything, returning the buffer.
    ///
    /// The one teardown path for every way a recording can end — stop, VAD,
    /// length cap, cancel, errors, and a new StartRecording replacing stale
    /// slots — so none of them can leave the mic open.
    ///
    /// `abort_stream` kills the WebSocket task without letting it flush. A
    /// normal stop leaves it running: the VU loop ends the stream by dropping
    /// its audio sender once it has forwarded the final samples.
    fn teardown(&mut self, abort_stream: bool) -> Option<PcmBuffer> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(task) = self.stream_task.take() {
            if abort_stream {
                task.abort();
            }
        }
        self.audio_buf.take()
    }
}

// ── Shared daemon core ────────────────────────────────────────────────────────

/// All daemon state, shared between the DBus interface and its background
/// tasks. Lock order is `state` → `capture` wherever both are held.
pub struct Core {
    state:       Mutex<StateCell>,
    capture:     Mutex<Capture>,
    config:      Mutex<AppConfig>,
    http_client: reqwest::Client,
    db:          Arc<Database>,
}

impl Core {
    fn config(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    fn is_current(&self, generation: u64) -> bool {
        self.state.lock().unwrap().generation == generation
    }

    fn is_recording(&self, generation: u64) -> bool {
        let st = self.state.lock().unwrap();
        st.generation == generation && st.state == DaemonState::Recording
    }

    /// Begin a new session: refuse if one is in flight, otherwise bump the
    /// generation and enter Recording. Returns the new generation.
    fn claim_start(&self, streaming: bool) -> Result<u64, String> {
        let mut st = self.state.lock().unwrap();
        if st.state.is_busy() {
            return Err("already recording or transcribing".into());
        }
        // Should already be empty; clearing it guards against a previous
        // session that ended on a path that forgot to tear down.
        self.capture.lock().unwrap().teardown(true);
        st.generation += 1;
        st.state = DaemonState::Recording;
        st.streaming = streaming;
        st.speech_detected = false;
        Ok(st.generation)
    }

    /// Store the capture handles, unless the session was cancelled while the
    /// device was opening — then stop the mic straight away.
    fn attach_capture(&self, generation: u64, stop_tx: watch::Sender<bool>, buf: PcmBuffer) -> bool {
        let st = self.state.lock().unwrap();
        if st.generation != generation || st.state != DaemonState::Recording {
            let _ = stop_tx.send(true);
            return false;
        }
        let mut cap = self.capture.lock().unwrap();
        cap.stop_tx = Some(stop_tx);
        cap.audio_buf = Some(buf);
        true
    }

    /// Store the WebSocket task handle, unless the session has already ended.
    /// A stop during the handshake is fine (state is then Streaming) — the
    /// buffered audio still gets sent and flushed.
    fn attach_stream(&self, generation: u64, task: tokio::task::AbortHandle) -> bool {
        let st = self.state.lock().unwrap();
        if st.generation != generation || !st.state.is_busy() {
            task.abort();
            return false;
        }
        self.capture.lock().unwrap().stream_task = Some(task);
        true
    }

    fn mark_speech(&self, generation: u64) {
        let mut st = self.state.lock().unwrap();
        if st.generation == generation {
            st.speech_detected = true;
        }
    }

    /// Move to `to` if `generation` is still current. Entering a terminal
    /// state tears down any capture the session still holds.
    fn transition(&self, generation: u64, to: DaemonState) -> bool {
        let mut st = self.state.lock().unwrap();
        if st.generation != generation {
            return false;
        }
        if to.is_terminal() {
            self.capture.lock().unwrap().teardown(true);
        }
        st.state = to;
        true
    }

    /// `transition`, then announce it. Returns false if the session is stale.
    async fn set_state(&self, emitter: &SignalEmitter<'_>, generation: u64, to: DaemonState) -> bool {
        let name = to.as_str().to_string();
        if !self.transition(generation, to) {
            return false;
        }
        emit_state(emitter, &name).await;
        true
    }

    /// Enter Error with a short, user-facing `message`, announcing it with
    /// ErrorOccurred just before StateChanged so clients can show *why*.
    async fn fail(&self, emitter: &SignalEmitter<'_>, generation: u64, message: String) {
        tracing::error!("{message}");
        if !self.transition(generation, DaemonState::Error(message.clone())) {
            return;
        }
        if let Err(e) = TranscriberInterface::error_occurred(emitter, &message).await {
            tracing::warn!("failed to emit ErrorOccurred: {e}");
        }
        emit_state(emitter, "Error").await;
    }

    /// End the recording and hand its audio to the transcription path.
    ///
    /// Shared by StopRecording, VAD auto-stop and the length cap. The
    /// Recording → Transcribing/Streaming compare-and-set happens under the
    /// state lock, so when two of them race exactly one wins and the others
    /// are no-ops. `expected` pins the stop to one session, so a lagging VU
    /// loop can never stop the recording that replaced its own.
    async fn stop(self: &Arc<Self>, emitter: &SignalEmitter<'_>, expected: Option<u64>, why: &str) {
        let (generation, streaming, speech_detected) = {
            let mut st = self.state.lock().unwrap();
            if st.state != DaemonState::Recording || expected.is_some_and(|g| g != st.generation) {
                return;
            }
            st.state = if st.streaming { DaemonState::Streaming } else { DaemonState::Transcribing };
            (st.generation, st.streaming, st.speech_detected)
        };
        let buf = self.capture.lock().unwrap().teardown(false);
        tracing::info!("recording stopped ({why})");

        if streaming {
            // The VU loop notices the state change, forwards the last samples
            // and drops its sender — that is the end-of-stream signal; the
            // stream consumer does the rest.
            emit_state(emitter, "Streaming").await;
            return;
        }

        let samples = buf.map(|b| std::mem::take(&mut *b.lock().unwrap())).unwrap_or_default();
        let cfg = self.config();
        let too_short = (samples.len() as f32) < cfg.sample_rate as f32 * MIN_RECORDING_SECS;
        if too_short || (cfg.vad_enabled && !speech_detected) {
            // Nothing worth a paid API call or a history row: Whisper tends
            // to hallucinate text ("Thank you.") out of pure silence.
            tracing::info!(
                "skipping transcription: {} samples, speech detected: {speech_detected}",
                samples.len()
            );
            self.set_state(emitter, generation, DaemonState::Idle).await;
            return;
        }

        emit_state(emitter, "Transcribing").await;
        tracing::info!("captured {} samples, starting transcription", samples.len());
        let core = self.clone();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            run_batch_pipeline(core, generation, emitter, cfg, samples).await;
        });
    }
}

/// Announce a state change via both the StateChanged signal and the standard
/// PropertiesChanged for `CurrentState`, so property-binding clients stay in
/// sync without subscribing to our custom signal.
async fn emit_state(emitter: &SignalEmitter<'_>, state: &str) {
    if let Err(e) = TranscriberInterface::state_changed(emitter, state).await {
        tracing::warn!("failed to emit StateChanged: {e}");
    }
    let changed = HashMap::from([("CurrentState", Value::from(state))]);
    if let Err(e) = zbus::fdo::Properties::properties_changed(
        emitter,
        InterfaceName::from_static_str_unchecked(common::dbus::INTERFACE_NAME),
        changed,
        Cow::Borrowed(&[]),
    )
    .await
    {
        tracing::warn!("failed to emit PropertiesChanged: {e}");
    }
}

// ── DBus interface struct ─────────────────────────────────────────────────────

pub struct TranscriberInterface {
    core: Arc<Core>,
}

impl TranscriberInterface {
    pub fn new(config: AppConfig, http_client: reqwest::Client, db: Arc<Database>) -> Self {
        Self {
            core: Arc::new(Core {
                state: Mutex::new(StateCell {
                    state:           DaemonState::Idle,
                    generation:      0,
                    streaming:       false,
                    speech_detected: false,
                }),
                capture: Mutex::new(Capture::default()),
                config: Mutex::new(config),
                http_client,
                db,
            }),
        }
    }
}

// ── zbus interface implementation ─────────────────────────────────────────────

// Every method takes `&self`: zbus holds the interface's RwLock for the whole
// call, and a `&mut self` method awaiting a network round-trip would block
// every other call (including Cancel) until it finished. All state lives in
// `Core` behind its own mutexes instead.
#[interface(name = "org.transcriber.Daemon")]
impl TranscriberInterface {

    // ── Signals ──────────────────────────────────────────────────────────────

    /// Emitted whenever the daemon state changes.
    /// Possible values: "Idle" | "Recording" | "Transcribing" | "PostProcessing" | "Streaming" | "Done" | "Error"
    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;

    /// Emitted immediately before StateChanged("Error") with a short
    /// human-readable reason, e.g. "Groq: 413 file too large".
    #[zbus(signal)]
    async fn error_occurred(emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;

    /// Emitted after a successful transcription with the resulting text.
    #[zbus(signal)]
    async fn transcription_ready(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()>;

    /// Emitted during streaming with each finalized phrase.
    /// The extension should type only the delta since the last chunk.
    #[zbus(signal)]
    async fn transcription_chunk(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()>;

    /// Current audio input level for VU meter display, in the range 0.0–1.0.
    /// Emitted approximately 30 times per second while recording.
    #[zbus(signal)]
    async fn audio_level(emitter: &SignalEmitter<'_>, level: f64) -> zbus::Result<()>;

    // ── Properties ───────────────────────────────────────────────────────────

    /// The daemon's current state string (same values as StateChanged signal).
    /// Changes are announced via PropertiesChanged as well.
    #[zbus(property)]
    async fn current_state(&self) -> String {
        self.core.state.lock().unwrap().state.as_str().to_string()
    }

    // ── Methods ──────────────────────────────────────────────────────────────

    /// Begin capturing audio from the default microphone.
    async fn start_recording(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let core = &self.core;
        let cfg = core.config();
        // Post-processing is mutually exclusive with streaming. If both are on
        // (e.g. the user manually edited config.json), prefer post-processing
        // and force the batch path.
        let streaming = cfg.streaming_enabled && !cfg.postprocess_enabled;

        let generation = core.claim_start(streaming).map_err(zbus::fdo::Error::Failed)?;
        let emitter = emitter.to_owned();

        if streaming {
            let problem = if !cfg.direct_injection {
                Some("streaming requires direct injection to be enabled")
            } else if cfg.active_streaming_key().is_empty() {
                Some("missing API key")
            } else {
                None
            };
            if let Some(problem) = problem {
                // Fail before opening the mic: otherwise the user would talk
                // into a recording that can only ever end in an auth error.
                let msg = format!("{}: {problem}", streaming_label(&cfg));
                core.fail(&emitter, generation, msg.clone()).await;
                return Err(zbus::fdo::Error::Failed(msg));
            }
        }

        let (buf, stop_tx) = match audio::start_capture(cfg.sample_rate) {
            Ok(c)  => c,
            Err(e) => {
                let msg = format!("Microphone: {e}");
                core.fail(&emitter, generation, msg.clone()).await;
                return Err(zbus::fdo::Error::Failed(msg));
            }
        };
        if !core.attach_capture(generation, stop_tx, buf.clone()) {
            return Ok(()); // cancelled while the device was opening
        }

        emit_state(&emitter, "Recording").await;
        if cfg.audio_cues_enabled {
            crate::audio_cues::play_start();
        }

        // Streaming: the channel exists before the WebSocket does, so audio
        // captured during the handshake is queued rather than lost.
        let forward = streaming.then(|| {
            let (tx, rx) = mpsc::unbounded_channel();
            tokio::spawn(run_stream_session(core.clone(), generation, emitter.clone(), cfg.clone(), rx));
            tx
        });
        tokio::spawn(run_level_loop(core.clone(), generation, emitter, cfg, buf, forward));

        Ok(())
    }

    /// Stop recording and send the audio to the transcription API.
    /// A no-op unless currently Recording (e.g. VAD already stopped it).
    async fn stop_recording(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.core.stop(&emitter, None, "StopRecording").await;
        Ok(())
    }

    /// Cancel the current session without delivering any text — whether it
    /// is still recording or already transcribing/streaming.
    async fn cancel(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        {
            let mut st = self.core.state.lock().unwrap();
            // Invalidate whatever is in flight; see `StateCell`.
            st.generation += 1;
            st.state = DaemonState::Idle;
            // Abort, not end-of-stream: dropping the audio sender would make
            // the provider flush and return the final transcript.
            self.core.capture.lock().unwrap().teardown(true);
        }
        emit_state(&emitter, "Idle").await;
        Ok(())
    }

    /// Return the most recent history entries as JSON.
    async fn get_history(&self, limit: u32) -> zbus::fdo::Result<String> {
        let db = self.core.db.clone();
        let entries = tokio::task::spawn_blocking(move || db.recent(limit))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let json: Vec<serde_json::Value> = entries
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "timestamp": e.timestamp,
                    "text": e.text,
                    "status": e.status,
                    "error": e.error,
                    "wav_path": e.wav_path,
                    "text_original": e.text_original,
                })
            })
            .collect();
        serde_json::to_string(&json).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Delete a single history entry by id, along with its WAV recording.
    async fn delete_history_entry(&self, id: i64) -> zbus::fdo::Result<()> {
        let db = self.core.db.clone();
        tokio::task::spawn_blocking(move || db.delete_entry(id))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Delete all history entries and their WAV recordings.
    async fn clear_history(&self) -> zbus::fdo::Result<()> {
        let db = self.core.db.clone();
        tokio::task::spawn_blocking(move || db.clear_all())
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Reload the in-memory config from `~/.config/transcriber/config.json`.
    /// Called by the settings app after the user changes a setting so the next
    /// recording uses the new values without requiring a daemon restart.
    /// A reload during an active recording does not affect that pipeline — the
    /// config is cloned at the moment recording stops, so changes take effect
    /// from the *following* recording onward.
    async fn reload_config(&self) -> zbus::fdo::Result<()> {
        let new_cfg = common::config::load()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        *self.core.config.lock().unwrap() = new_cfg;
        tracing::info!("config reloaded from disk");
        Ok(())
    }

    /// Re-transcribe (and post-process, if enabled) a history entry's saved
    /// WAV. Updates the entry in the database and returns the final text.
    /// Returns a DBus error on failure (the history entry is updated with the new error).
    ///
    /// `wav_path` is **ignored** and kept only for signature compatibility:
    /// the recording is looked up from the database by `history_id`, so a
    /// DBus caller cannot make the daemon read (and upload) arbitrary files.
    async fn retry_transcription(
        &self,
        history_id: i64,
        wav_path: String,
    ) -> zbus::fdo::Result<String> {
        let _ = wav_path;
        let failed = |m: String| zbus::fdo::Error::Failed(m);

        let db = self.core.db.clone();
        let stored = tokio::task::spawn_blocking(move || db.wav_path(history_id))
            .await
            .map_err(|e| failed(e.to_string()))?
            .map_err(|e| failed(e.to_string()))?
            .ok_or_else(|| failed(format!("history entry {history_id} has no saved recording")))?;
        let path = crate::db::recording_path(Path::new(&stored))
            .ok_or_else(|| failed(format!("recording {stored} is missing or outside the recordings directory")))?;
        let wav = tokio::fs::read(&path)
            .await
            .map_err(|e| failed(format!("could not read WAV file: {e}")))?;

        let cfg = self.core.config();
        let db  = self.core.db.clone();

        match transcribe_and_polish(&self.core.http_client, &cfg, &wav, async {}).await {
            Ok((text, original)) => {
                let text_clone = text.clone();
                tokio::task::spawn_blocking(move || {
                    db.update_retry_success(history_id, &text_clone, original.as_deref())
                })
                .await
                .ok();
                tracing::info!("retry transcription {history_id} done: {} chars", text.len());
                Ok(text)
            }
            Err(e) => {
                let err_str = e.to_string();
                tokio::task::spawn_blocking(move || db.update_retry_failed(history_id, &err_str))
                    .await
                    .ok();
                tracing::warn!("retry transcription {history_id} failed: {e}");
                Err(failed(describe_error(&cfg, &e)))
            }
        }
    }
}

// ── Background tasks ──────────────────────────────────────────────────────────

/// While recording: emit AudioLevel at ~30 Hz for the VU meter, track speech
/// for VAD auto-stop and empty-recording detection, enforce the length cap,
/// and — when `forward` is set (streaming) — send each new chunk of audio to
/// the WebSocket.
///
/// On streaming, exiting this loop is what ends the stream: after the stop it
/// forwards the final samples and drops `forward`, which the WebSocket task
/// sees as end-of-stream.
async fn run_level_loop(
    core: Arc<Core>,
    generation: u64,
    emitter: SignalEmitter<'static>,
    cfg: AppConfig,
    buf: PcmBuffer,
    forward: Option<mpsc::UnboundedSender<Vec<i16>>>,
) {
    let interval = Duration::from_millis(33);
    let started  = Instant::now();
    let max_len  = (cfg.max_recording_secs > 0)
        .then(|| Duration::from_secs(u64::from(cfg.max_recording_secs)));
    let mut prev_len = 0usize;

    let mut speech_detected = false;
    let mut silence_ticks   = 0usize;
    const SILENCE_THRESHOLD_TICKS: usize = 45; // 45 × 33 ms ≈ 1.5 s
    const SILENCE_LEVEL_CUTOFF:    f64   = 0.05;

    loop {
        tokio::time::sleep(interval).await;

        // Checked before reading the buffer so the final read below still
        // picks up everything captured before the stop.
        let recording = core.is_recording(generation);

        let (rms, chunk) = {
            let guard = buf.lock().unwrap();
            // `get` rather than indexing: the batch stop path empties the
            // buffer with `mem::take`, which can land between our reads.
            let samples = guard.get(prev_len..).unwrap_or(&[]);
            prev_len = prev_len.max(guard.len());
            if samples.is_empty() {
                (0.0f64, None)
            } else {
                let rms = (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>()
                    / samples.len() as f64)
                    .sqrt();
                (rms, forward.is_some().then(|| streaming::f32_to_i16(samples)))
            }
        };

        if let (Some(tx), Some(chunk)) = (&forward, chunk) {
            // Only fails once the WebSocket task is gone, when there's
            // nowhere left to send to anyway.
            let _ = tx.send(chunk);
        }

        if !recording {
            break;
        }

        // Read live rather than from `cfg` so the settings app's gain/floor
        // sliders take effect on the VU meter immediately.
        let (gain, floor) = {
            let c = core.config.lock().unwrap();
            (c.audio_level_gain, c.audio_level_floor)
        };
        let level = ((rms - floor).max(0.0) * gain).clamp(0.0, 1.0);

        if let Err(e) = TranscriberInterface::audio_level(&emitter, level).await {
            tracing::warn!("failed to emit AudioLevel: {e}");
        }

        if level >= SILENCE_LEVEL_CUTOFF {
            if !speech_detected {
                core.mark_speech(generation);
            }
            speech_detected = true;
            silence_ticks   = 0;
        } else if speech_detected {
            silence_ticks += 1;
        }

        if cfg.vad_enabled && silence_ticks >= SILENCE_THRESHOLD_TICKS {
            core.stop(&emitter, Some(generation), "VAD silence").await;
        } else if max_len.is_some_and(|m| started.elapsed() >= m) {
            core.stop(&emitter, Some(generation), "maximum recording length").await;
        }
    }
}

/// Connect the streaming WebSocket, then relay its transcript to DBus.
async fn run_stream_session(
    core: Arc<Core>,
    generation: u64,
    emitter: SignalEmitter<'static>,
    cfg: AppConfig,
    audio_rx: streaming::AudioRx,
) {
    let label = streaming_label(&cfg);
    let session = match cfg.streaming_provider.as_str() {
        "assemblyai" => {
            streaming::start_assemblyai_session(
                &cfg.assemblyai_api_key,
                cfg.sample_rate,
                cfg.active_language(),
                audio_rx,
            ).await
        }
        _ => {
            streaming::start_deepgram_session(
                &cfg.deepgram_api_key,
                &cfg.deepgram_model,
                cfg.sample_rate,
                cfg.active_language(),
                audio_rx,
            ).await
        }
    };
    let mut session = match session {
        Ok(s)  => s,
        Err(e) => {
            // `fail` also tears down the capture, so the mic doesn't keep
            // running into a stream that will never exist.
            core.fail(&emitter, generation, format!("{label}: {e:#}")).await;
            return;
        }
    };
    if !core.attach_stream(generation, session.task.abort_handle()) {
        return; // cancelled during the handshake
    }

    let mut accumulated = String::new();
    loop {
        match session.event_rx.recv().await {
            Some(StreamEvent::Final(phrase)) => {
                tracing::debug!("streaming final phrase: {:?}", phrase);
                if !core.is_current(generation) {
                    return;
                }
                if !accumulated.is_empty() { accumulated.push(' '); }
                accumulated.push_str(&phrase);
                // Emit accumulated text so extension can compute delta
                let _ = TranscriberInterface::transcription_chunk(&emitter, &accumulated).await;
            }
            // `None` without `Done` means the task was aborted by Cancel,
            // which has already bumped the generation.
            Some(StreamEvent::Done) | None => {
                tracing::info!("streaming done: {} chars", accumulated.len());
                if accumulated.is_empty() {
                    core.set_state(&emitter, generation, DaemonState::Idle).await;
                } else if deliver(&core, generation, &emitter, &cfg, accumulated, None, None).await {
                    core.set_state(&emitter, generation, DaemonState::Done).await;
                }
                return;
            }
            Some(StreamEvent::Error(e)) => {
                // Text already typed via chunks is real; keep it in history
                // and hand it over rather than losing it with the error.
                if !accumulated.is_empty() {
                    deliver(&core, generation, &emitter, &cfg, accumulated, None, None).await;
                }
                core.fail(&emitter, generation, format!("{label}: {e}")).await;
                return;
            }
        }
    }
}

/// Encode samples to WAV, transcribe, optionally post-process through an LLM,
/// persist to history, and emit the final-state DBus signals.
///
/// Drives the `Transcribing → [PostProcessing →] Done` portion of the state
/// machine for both manual stop and auto-stop. Everything it announces is
/// gated on `generation`, so a Cancel mid-flight makes it finish silently.
async fn run_batch_pipeline(
    core: Arc<Core>,
    generation: u64,
    emitter: SignalEmitter<'static>,
    cfg: AppConfig,
    samples: Vec<f32>,
) {
    let wav = match audio::encode_wav(&samples, cfg.sample_rate) {
        Ok(w)  => w,
        Err(e) => {
            core.fail(&emitter, generation, format!("Could not encode audio: {e}")).await;
            return;
        }
    };
    drop(samples);

    let to_postprocessing = core.set_state(&emitter, generation, DaemonState::PostProcessing);
    let result = transcribe_and_polish(&core.http_client, &cfg, &wav, async {
        to_postprocessing.await;
    })
    .await;

    // Only now write the WAV: a cancelled session must leave nothing behind,
    // and a WAV with no history row pointing at it would never be cleaned up.
    let wav_path = || -> Option<String> {
        if !cfg.save_history || !core.is_current(generation) {
            return None;
        }
        match save_wav_to_disk(&wav) {
            Ok(p)  => Some(p.to_string_lossy().into_owned()),
            Err(e) => {
                tracing::warn!("failed to save WAV to disk: {e}");
                None
            }
        }
    };

    match result {
        Ok((text, _)) if text.is_empty() => {
            tracing::info!("transcription came back empty");
            core.set_state(&emitter, generation, DaemonState::Idle).await;
        }
        Ok((text, original)) => {
            let path = wav_path();
            if deliver(&core, generation, &emitter, &cfg, text, original, path).await {
                core.set_state(&emitter, generation, DaemonState::Done).await;
            }
        }
        Err(e) => {
            tracing::error!("transcription failed: {e}");
            if cfg.save_history && core.is_current(generation) {
                let err_str = e.to_string();
                let path = wav_path();
                save_history(&core, &cfg, move |db| db.insert_failed(&err_str, path.as_deref())).await;
            }
            core.fail(&emitter, generation, describe_error(&cfg, &e)).await;
        }
    }
}

/// Emit TranscriptionReady, play the done cue and write history — if the
/// session is still current. Returns false if it was cancelled.
async fn deliver(
    core: &Arc<Core>,
    generation: u64,
    emitter: &SignalEmitter<'_>,
    cfg: &AppConfig,
    text: String,
    original: Option<String>,
    wav_path: Option<String>,
) -> bool {
    if !core.is_current(generation) {
        return false;
    }
    let _ = TranscriberInterface::transcription_ready(emitter, &text).await;
    if cfg.audio_cues_enabled {
        crate::audio_cues::play_done();
    }
    if cfg.save_history {
        save_history(core, cfg, move |db| {
            db.insert(&text, original.as_deref(), wav_path.as_deref())
        })
        .await;
    }
    true
}

/// Run a history write, then apply the retention limit.
async fn save_history(
    core: &Core,
    cfg: &AppConfig,
    write: impl FnOnce(&Database) -> Result<()> + Send + 'static,
) {
    let db = core.db.clone();
    let keep = cfg.history_max_entries;
    let res = tokio::task::spawn_blocking(move || {
        write(&db)?;
        db.prune(keep)
    })
    .await;
    match res {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("failed to write history: {e}"),
        Err(e)     => tracing::warn!("history task failed: {e}"),
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Save WAV bytes to `~/.local/share/transcriber/recordings/<millis>.wav`.
/// Returns the absolute path on success.
fn save_wav_to_disk(wav: &[u8]) -> Result<std::path::PathBuf> {
    let dir = common::config::data_dir().join("recordings");
    std::fs::create_dir_all(&dir)?;

    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let path = dir.join(format!("{ms}.wav"));
    std::fs::write(&path, wav)?;
    Ok(path)
}

fn provider_label(cfg: &AppConfig) -> &'static str {
    match cfg.provider.as_str() {
        "groq"   => "Groq",
        "cohere" => "Cohere",
        _        => "Custom endpoint",
    }
}

fn streaming_label(cfg: &AppConfig) -> &'static str {
    match cfg.streaming_provider.as_str() {
        "assemblyai" => "AssemblyAI",
        _            => "Deepgram",
    }
}

/// Short user-facing form of a transcription error, e.g. "Groq: 413 file too
/// large". The full error (with response body) goes to logs and history.
fn describe_error(cfg: &AppConfig, e: &anyhow::Error) -> String {
    let detail = if let Some(api) = e.downcast_ref::<ApiError>() {
        api.short()
    } else if let Some(re) = e.downcast_ref::<reqwest::Error>() {
        if re.is_timeout() {
            "request timed out".into()
        } else if re.is_connect() {
            "could not connect".into()
        } else {
            re.to_string()
        }
    } else {
        e.to_string()
    };
    format!("{}: {detail}", provider_label(cfg))
}

/// Returns true for errors that may be transient and worth retrying.
fn is_retryable(e: &anyhow::Error) -> bool {
    if let Some(api) = e.downcast_ref::<ApiError>() {
        return api.is_retryable();
    }
    if let Some(re) = e.downcast_ref::<reqwest::Error>() {
        return re.is_connect() || re.is_timeout() || re.is_request();
    }
    false
}

/// Longest `Retry-After` we'll honour. Anything longer means a quota reset,
/// not a blip, and the user is better served by an error they can retry.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);

/// Call the transcription API, retrying up to 2 more times on transient failures.
async fn transcribe_with_retry(
    client: &reqwest::Client,
    cfg: &AppConfig,
    wav: &[u8],
) -> Result<String> {
    // Custom endpoints may legitimately be keyless (a local whisper server).
    if cfg.active_key().is_empty() && matches!(cfg.provider.as_str(), "groq" | "cohere") {
        return Err(anyhow!("missing API key"));
    }

    let mut attempt = 0u32;
    loop {
        let err = match crate::api::transcribe(
            client,
            cfg.active_url(),
            cfg.active_key(),
            cfg.active_model(),
            cfg.active_language(),
            cfg.active_prompt().as_deref(),
            wav.to_vec(),
        )
        .await
        {
            Ok(text) => return Ok(text),
            Err(e)   => e,
        };

        attempt += 1;
        if attempt >= 3 || !is_retryable(&err) {
            return Err(err);
        }
        let delay = err
            .downcast_ref::<ApiError>()
            .and_then(|a| a.retry_after)
            .map(|d| d.min(MAX_RETRY_AFTER))
            .unwrap_or(Duration::from_millis(600 * u64::from(attempt)));
        tracing::warn!(
            "transient transcription error (attempt {attempt}), retrying in {:.1}s: {err}",
            delay.as_secs_f32()
        );
        tokio::time::sleep(delay).await;
    }
}

/// Transcribe `wav` and, if enabled, run it through the post-processing LLM.
///
/// Returns `(final_text, text_original)` where `text_original` is the raw
/// transcript when post-processing replaced it, else `None` — the shape
/// `Database::insert` expects. `before_polish` is awaited only if the
/// post-processing pass actually runs (the live pipeline uses it to announce
/// the PostProcessing state; retries pass a no-op). A failed polish falls
/// back to the raw text rather than losing the transcription.
async fn transcribe_and_polish(
    client: &reqwest::Client,
    cfg: &AppConfig,
    wav: &[u8],
    before_polish: impl Future<Output = ()>,
) -> Result<(String, Option<String>)> {
    let raw = transcribe_with_retry(client, cfg, wav).await?;
    tracing::info!("transcription done: {} chars", raw.len());

    if !cfg.postprocess_enabled || raw.is_empty() {
        return Ok((raw, None));
    }

    before_polish.await;
    match crate::api::postprocess(
        client,
        cfg.active_postprocess_url(),
        cfg.active_postprocess_key(),
        cfg.active_postprocess_model(),
        &cfg.active_postprocess_system_prompt(),
        &raw,
    )
    .await
    {
        Ok(polished) => {
            tracing::info!("post-processing done: {} chars", polished.len());
            Ok((polished, Some(raw)))
        }
        Err(e) => {
            tracing::warn!("post-processing failed, falling back to raw: {e}");
            Ok((raw, None))
        }
    }
}

// ── Connection setup (called from main) ──────────────────────────────────────

pub async fn build_connection(iface: TranscriberInterface) -> Result<Connection> {
    let conn = zbus::connection::Builder::session()?
        .name(common::dbus::SERVICE_NAME)?
        .serve_at(common::dbus::OBJECT_PATH, iface)?
        .build()
        .await?;
    Ok(conn)
}
