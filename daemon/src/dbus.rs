use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;
use zbus::{interface, object_server::SignalEmitter, Connection};

use crate::audio::{self, PcmBuffer};
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
}

// ── DBus interface struct ─────────────────────────────────────────────────────

pub struct TranscriberInterface {
    pub state:       Arc<Mutex<DaemonState>>,
    pub config:      Arc<Mutex<AppConfig>>,
    /// The live audio buffer while recording; None otherwise.
    pub audio_buf:   Arc<Mutex<Option<PcmBuffer>>>,
    /// Sender to stop the cpal capture thread; None when not recording.
    pub stop_tx:     Arc<Mutex<Option<tokio::sync::watch::Sender<bool>>>>,
    /// Sender to forward audio chunks to an active streaming session; None otherwise.
    pub stream_audio_tx: Arc<Mutex<Option<mpsc::Sender<Vec<i16>>>>>,
    pub http_client: reqwest::Client,
    pub db:          Arc<crate::db::Database>,
}

// ── zbus interface implementation ─────────────────────────────────────────────

#[interface(name = "org.transcriber.Daemon")]
impl TranscriberInterface {

    // ── Signals ──────────────────────────────────────────────────────────────

    /// Emitted whenever the daemon state changes.
    /// Possible values: "Idle" | "Recording" | "Transcribing" | "PostProcessing" | "Streaming" | "Done" | "Error"
    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;

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
    #[zbus(property)]
    async fn current_state(&self) -> String {
        self.state.lock().unwrap().as_str().to_string()
    }

    // ── Methods ──────────────────────────────────────────────────────────────

    /// Begin capturing audio from the default microphone.
    async fn start_recording(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        {
            let state = self.state.lock().unwrap();
            if matches!(
                *state,
                DaemonState::Recording
                    | DaemonState::Transcribing
                    | DaemonState::PostProcessing
                    | DaemonState::Streaming
            ) {
                return Err(zbus::fdo::Error::Failed(
                    "already recording or transcribing".into(),
                ));
            }
        }

        let sample_rate      = self.config.lock().unwrap().sample_rate;
        let vad_enabled      = self.config.lock().unwrap().vad_enabled;
        // Post-processing is mutually exclusive with streaming. If both are on
        // (e.g. the user manually edited config.json), prefer post-processing
        // and force the batch path.
        let postprocess_enabled = self.config.lock().unwrap().postprocess_enabled;
        let streaming_enabled   = self.config.lock().unwrap().streaming_enabled && !postprocess_enabled;
        let direct_injection    = self.config.lock().unwrap().direct_injection;

        if streaming_enabled && !direct_injection {
            return Err(zbus::fdo::Error::Failed(
                "streaming transcription requires direct injection to be enabled".into(),
            ));
        }

        let (buf, stop_tx) =
            audio::start_capture(sample_rate).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        *self.audio_buf.lock().unwrap() = Some(buf.clone());
        *self.stop_tx.lock().unwrap() = Some(stop_tx);
        *self.state.lock().unwrap() = DaemonState::Recording;

        Self::state_changed(&emitter, "Recording").await?;

        if self.config.lock().unwrap().audio_cues_enabled {
            crate::audio_cues::play_start();
        }

        if streaming_enabled {
            // ── Streaming path ──────────────────────────────────────────────
            let cfg = self.config.lock().unwrap().clone();
            let session = match cfg.streaming_provider.as_str() {
                "assemblyai" => {
                    streaming::start_assemblyai_session(
                        &cfg.assemblyai_api_key,
                        cfg.sample_rate,
                        cfg.active_language(),
                    ).await
                }
                _ => {
                    streaming::start_deepgram_session(
                        &cfg.deepgram_api_key,
                        &cfg.deepgram_model,
                        cfg.sample_rate,
                        cfg.active_language(),
                    ).await
                }
            };
            let session = match session {
                Ok(s)  => s,
                Err(e) => {
                    tracing::error!("failed to start streaming session: {e}");
                    *self.state.lock().unwrap() = DaemonState::Idle;
                    return Err(zbus::fdo::Error::Failed(e.to_string()));
                }
            };

            *self.stream_audio_tx.lock().unwrap() = Some(session.audio_tx);

            // Spawn audio-forward + VU meter + VAD task
            let stream_audio_tx = self.stream_audio_tx.clone();
            let stop_tx_vad     = self.stop_tx.clone();
            let audio_buf_vad   = self.audio_buf.clone();
            let state_watch     = self.state.clone();
            let emitter_owned   = emitter.to_owned();
            tokio::spawn(async move {
                let interval = Duration::from_millis(33);
                let mut prev_len = 0usize;

                let mut speech_detected = false;
                let mut silence_ticks   = 0usize;
                const SILENCE_THRESHOLD_TICKS: usize = 45;
                const SILENCE_LEVEL_CUTOFF:    f64   = 0.05;

                loop {
                    tokio::time::sleep(interval).await;

                    if *state_watch.lock().unwrap() != DaemonState::Recording {
                        break;
                    }

                    let (level, new_samples) = {
                        let guard = buf.lock().unwrap();
                        let samples = &guard[prev_len..];
                        if samples.is_empty() {
                            (0.0f64, vec![])
                        } else {
                            let rms = (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>()
                                / samples.len() as f64)
                                .sqrt();
                            let new_i16 = streaming::f32_to_i16(samples);
                            prev_len = guard.len();
                            // VU mapping: subtract a ~-34dB noise floor
                            // then scale. The previous (rms * 25.0) had
                            // no floor and saturated on any room ambient.
                            let scaled = ((rms - 0.02).max(0.0) * 10.0).clamp(0.0, 1.0);
                            (scaled, new_i16)
                        }
                    };

                    // Forward audio chunk to WebSocket
                    if !new_samples.is_empty() {
                        let tx_guard = stream_audio_tx.lock().unwrap();
                        if let Some(tx) = tx_guard.as_ref() {
                            let _ = tx.try_send(new_samples);
                        }
                    }

                    if let Err(e) = Self::audio_level(&emitter_owned, level).await {
                        tracing::warn!("failed to emit AudioLevel: {e}");
                        break;
                    }

                    if vad_enabled {
                        if level >= SILENCE_LEVEL_CUTOFF {
                            speech_detected = true;
                            silence_ticks   = 0;
                        } else if speech_detected {
                            silence_ticks += 1;
                            if silence_ticks >= SILENCE_THRESHOLD_TICKS {
                                // VAD triggered: drop audio_tx → end-of-stream
                                if let Some(tx) = stop_tx_vad.lock().unwrap().take() {
                                    let _ = tx.send(true);
                                }
                                audio_buf_vad.lock().unwrap().take();
                                stream_audio_tx.lock().unwrap().take(); // drops sender
                                {
                                    let mut st = state_watch.lock().unwrap();
                                    if *st != DaemonState::Recording { break; }
                                    *st = DaemonState::Streaming;
                                }
                                let _ = Self::state_changed(&emitter_owned, "Streaming").await;
                                tracing::info!("VAD triggered streaming end-of-stream");
                                break;
                            }
                        }
                    }
                }
            });

            // Spawn event-consumer task
            let mut event_rx   = session.event_rx;
            let state_ev       = self.state.clone();
            let db_ev          = self.db.clone();
            let emitter_ev     = emitter.to_owned();
            tokio::spawn(async move {
                let mut accumulated = String::new();
                loop {
                    match event_rx.recv().await {
                        Some(StreamEvent::Final(phrase)) => {
                            tracing::debug!("streaming final phrase: {:?}", phrase);
                            if !accumulated.is_empty() { accumulated.push(' '); }
                            accumulated.push_str(&phrase);
                            // Emit accumulated text so extension can compute delta
                            let _ = Self::transcription_chunk(&emitter_ev, &accumulated).await;
                        }
                        Some(StreamEvent::Done) | None => {
                            tracing::info!("streaming done: {} chars", accumulated.len());
                            if !accumulated.is_empty() {
                                let _ = Self::transcription_ready(&emitter_ev, &accumulated).await;
                                if cfg.audio_cues_enabled {
                                    crate::audio_cues::play_done();
                                }
                                if cfg.save_history {
                                    let text_clone = accumulated.clone();
                                    let db = db_ev.clone();
                                    tokio::task::spawn_blocking(move || {
                                        db.insert(&text_clone, None, None)
                                    }).await.ok();
                                }
                            }
                            *state_ev.lock().unwrap() = DaemonState::Done;
                            let _ = Self::state_changed(&emitter_ev, "Done").await;
                            break;
                        }
                        Some(StreamEvent::Error(e)) => {
                            tracing::error!("streaming error: {e}");
                            *state_ev.lock().unwrap() = DaemonState::Error(e.clone());
                            let _ = Self::state_changed(&emitter_ev, "Error").await;
                            break;
                        }
                    }
                }
            });
        } else {
            // ── Batch path (original) ────────────────────────────────────────

            // Clone Arcs needed by the VAD auto-stop path.
            let stop_tx_vad   = self.stop_tx.clone();
            let audio_buf_vad = self.audio_buf.clone();
            let config_vad    = self.config.clone();
            let client_vad    = self.http_client.clone();
            let db_vad        = self.db.clone();

            // Spawn a task that periodically reads the buffer tail, computes RMS,
            // and emits AudioLevel signals at ~30 Hz for a VU meter.
            // When VAD is enabled it also monitors for silence and auto-triggers
            // stop+transcribe after ~1.5 s of silence following detected speech.
            let state_watch = self.state.clone();
            let emitter_owned = emitter.to_owned();
            tokio::spawn(async move {
                let interval = Duration::from_millis(33);
                let mut prev_len = 0usize;

                let mut speech_detected = false;
                let mut silence_ticks   = 0usize;
                const SILENCE_THRESHOLD_TICKS: usize = 45; // 45 × 33 ms ≈ 1.5 s
                const SILENCE_LEVEL_CUTOFF:    f64   = 0.05;

                loop {
                    tokio::time::sleep(interval).await;

                    if *state_watch.lock().unwrap() != DaemonState::Recording {
                        break;
                    }

                    let level = {
                        let guard = buf.lock().unwrap();
                        let samples = &guard[prev_len..];
                        if samples.is_empty() {
                            0.0f64
                        } else {
                            let rms = (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>()
                                / samples.len() as f64)
                                .sqrt();
                            prev_len = guard.len();
                            // VU mapping: see comment in the streaming
                            // branch above. Same formula here so both
                            // capture paths render identically.
                            ((rms - 0.02).max(0.0) * 10.0).clamp(0.0, 1.0)
                        }
                    };

                    if let Err(e) = Self::audio_level(&emitter_owned, level).await {
                        tracing::warn!("failed to emit AudioLevel: {e}");
                        break;
                    }

                    if vad_enabled {
                        if level >= SILENCE_LEVEL_CUTOFF {
                            speech_detected = true;
                            silence_ticks   = 0;
                        } else if speech_detected {
                            silence_ticks += 1;
                            if silence_ticks >= SILENCE_THRESHOLD_TICKS {
                                // Step 1: stop audio capture
                                if let Some(tx) = stop_tx_vad.lock().unwrap().take() {
                                    let _ = tx.send(true);
                                }
                                // Step 2: take buffer
                                let samples: Vec<f32> = audio_buf_vad
                                    .lock().unwrap().take()
                                    .map(|b| b.lock().unwrap().clone())
                                    .unwrap_or_default();
                                // Step 3: transition state (guard against race with manual stop)
                                {
                                    let mut st = state_watch.lock().unwrap();
                                    if *st != DaemonState::Recording { break; }
                                    *st = DaemonState::Transcribing;
                                }
                                let _ = Self::state_changed(&emitter_owned, "Transcribing").await;
                                tracing::info!(
                                    "VAD triggered: {} samples, starting transcription",
                                    samples.len()
                                );
                                // Step 4: spawn transcription (mirrors stop_recording)
                                let cfg     = config_vad.lock().unwrap().clone();
                                let client  = client_vad.clone();
                                let db      = db_vad.clone();
                                let state   = state_watch.clone();
                                let emitter = emitter_owned.clone();
                                tokio::spawn(async move {
                                    run_batch_pipeline(client, cfg, samples, db, state, emitter).await;
                                });
                                break;
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Stop recording and send the audio to the transcription API.
    async fn stop_recording(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if let Some(tx) = self.stop_tx.lock().unwrap().take() {
            let _ = tx.send(true);
        }

        // Streaming path: drop audio_tx to signal end-of-stream; the event-consumer
        // task handles the rest and will emit TranscriptionReady when done.
        if self.stream_audio_tx.lock().unwrap().take().is_some() {
            self.audio_buf.lock().unwrap().take();
            *self.state.lock().unwrap() = DaemonState::Streaming;
            Self::state_changed(&emitter, "Streaming").await?;
            tracing::info!("streaming: end-of-stream signalled, waiting for final results");
            return Ok(());
        }

        // Batch path (original)
        let samples: Vec<f32> = self
            .audio_buf
            .lock()
            .unwrap()
            .take()
            .map(|b| b.lock().unwrap().clone())
            .unwrap_or_default();

        *self.state.lock().unwrap() = DaemonState::Transcribing;
        Self::state_changed(&emitter, "Transcribing").await?;

        tracing::info!("captured {} samples, starting transcription", samples.len());

        let cfg        = self.config.lock().unwrap().clone();
        let client     = self.http_client.clone();
        let db         = self.db.clone();
        let state      = self.state.clone();
        let emitter    = emitter.to_owned();

        tokio::spawn(async move {
            run_batch_pipeline(client, cfg, samples, db, state, emitter).await;
        });

        Ok(())
    }

    /// Cancel an in-progress recording without transcribing.
    async fn cancel(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if let Some(tx) = self.stop_tx.lock().unwrap().take() {
            let _ = tx.send(true);
        }
        // Drop the streaming sender without sending an end-of-stream message so
        // the event-consumer task exits without emitting TranscriptionReady.
        self.stream_audio_tx.lock().unwrap().take();
        *self.audio_buf.lock().unwrap() = None;
        *self.state.lock().unwrap() = DaemonState::Idle;
        Self::state_changed(&emitter, "Idle").await?;
        Ok(())
    }

    /// Return the most recent history entries as JSON.
    async fn get_history(&self, limit: u32) -> zbus::fdo::Result<String> {
        let db = self.db.clone();
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

    /// Delete a single history entry by id.
    async fn delete_history_entry(&self, id: i64) -> zbus::fdo::Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || db.delete_entry(id))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Delete all history entries.
    async fn clear_history(&self) -> zbus::fdo::Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || db.clear_all())
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Reload the in-memory config from `~/.config/voice-transcriber/config.json`.
    /// Called by the settings app after the user changes a setting so the next
    /// recording uses the new values without requiring a daemon restart.
    /// A reload during an active recording does not affect that pipeline — the
    /// config is cloned at the moment recording stops, so changes take effect
    /// from the *following* recording onward.
    async fn reload_config(&mut self) -> zbus::fdo::Result<()> {
        let new_cfg = common::config::load()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        *self.config.lock().unwrap() = new_cfg;
        tracing::info!("config reloaded from disk");
        Ok(())
    }

    /// Re-transcribe a previously saved WAV file from the history.
    /// Updates the history entry in the database and returns the transcribed text.
    /// Returns a DBus error on failure (the history entry is updated with the new error).
    async fn retry_transcription(
        &mut self,
        history_id: i64,
        wav_path: String,
    ) -> zbus::fdo::Result<String> {
        let wav = std::fs::read(&wav_path)
            .map_err(|e| zbus::fdo::Error::Failed(format!("could not read WAV file: {e}")))?;

        let cfg    = self.config.lock().unwrap().clone();
        let client = self.http_client.clone();
        let db     = self.db.clone();

        match transcribe_with_retry(&client, &cfg, wav).await {
            Ok(text) => {
                let text_clone = text.clone();
                let db2 = db.clone();
                tokio::task::spawn_blocking(move || db2.update_retry_success(history_id, &text_clone))
                    .await
                    .ok();
                tracing::info!("retry transcription {history_id} done: {} chars", text.len());
                Ok(text)
            }
            Err(e) => {
                let err_str = e.to_string();
                let err_clone = err_str.clone();
                tokio::task::spawn_blocking(move || db.update_retry_failed(history_id, &err_clone))
                    .await
                    .ok();
                tracing::warn!("retry transcription {history_id} failed: {err_str}");
                Err(zbus::fdo::Error::Failed(err_str))
            }
        }
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Save WAV bytes to `~/.local/share/voice-transcriber/recordings/<millis>.wav`.
/// Returns the absolute path on success.
fn save_wav_to_disk(wav: &[u8]) -> Result<std::path::PathBuf> {
    let dir = dirs::data_local_dir()
        .expect("no data dir")
        .join("voice-transcriber")
        .join("recordings");
    std::fs::create_dir_all(&dir)?;

    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let path = dir.join(format!("{ms}.wav"));
    std::fs::write(&path, wav)?;
    Ok(path)
}

/// Returns true for errors that may be transient and worth retrying.
fn is_retryable(e: &anyhow::Error) -> bool {
    if let Some(re) = e.downcast_ref::<reqwest::Error>() {
        if re.is_connect() || re.is_timeout() || re.is_request() {
            return true;
        }
        if let Some(status) = re.status() {
            return status.is_server_error();
        }
    }
    false
}

/// Call the transcription API, retrying up to 2 more times on transient failures.
async fn transcribe_with_retry(
    client: &reqwest::Client,
    cfg: &AppConfig,
    wav: Vec<u8>,
) -> Result<String> {
    let mut last_err: anyhow::Error = anyhow::anyhow!("no attempts made");

    for attempt in 0u32..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(600 * attempt as u64)).await;
            tracing::info!("retrying transcription (attempt {})", attempt + 1);
        }

        match crate::api::transcribe(
            client,
            cfg.active_url(),
            cfg.active_key(),
            cfg.active_model(),
            cfg.active_language(),
            wav.clone(),
        )
        .await
        {
            Ok(text) => return Ok(text),
            Err(e) => {
                if !is_retryable(&e) {
                    return Err(e);
                }
                tracing::warn!("transient transcription error (attempt {}): {e}", attempt + 1);
                last_err = e;
            }
        }
    }

    Err(last_err)
}

/// Encode samples to WAV, transcribe, optionally post-process through an LLM,
/// persist to history, and emit the final-state DBus signals.
///
/// Drives the full `Transcribing → [PostProcessing →] Done` portion of the
/// state machine. Used by both the manual-stop and VAD-auto-stop batch paths.
async fn run_batch_pipeline(
    client: reqwest::Client,
    cfg: AppConfig,
    samples: Vec<f32>,
    db: Arc<crate::db::Database>,
    state: Arc<Mutex<DaemonState>>,
    emitter: SignalEmitter<'static>,
) {
    let wav = match audio::encode_wav(&samples, cfg.sample_rate) {
        Ok(w)  => w,
        Err(e) => {
            tracing::error!("failed to encode WAV: {e}");
            *state.lock().unwrap() = DaemonState::Error(e.to_string());
            let _ = TranscriberInterface::state_changed(&emitter, "Error").await;
            return;
        }
    };

    let wav_path: Option<String> = if cfg.save_history {
        match save_wav_to_disk(&wav) {
            Ok(p)  => Some(p.to_string_lossy().into_owned()),
            Err(e) => {
                tracing::warn!("failed to save WAV to disk: {e}");
                None
            }
        }
    } else {
        None
    };

    let raw = match transcribe_with_retry(&client, &cfg, wav).await {
        Ok(t)  => t,
        Err(e) => {
            tracing::error!("transcription failed: {e}");
            if cfg.save_history {
                let err_str       = e.to_string();
                let wav_path_copy = wav_path.clone();
                let db2           = db.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    db2.insert_failed(&err_str, wav_path_copy.as_deref())
                })
                .await;
            }
            *state.lock().unwrap() = DaemonState::Error(e.to_string());
            let _ = TranscriberInterface::state_changed(&emitter, "Error").await;
            return;
        }
    };

    tracing::info!("transcription done: {} chars", raw.len());

    // Optional post-processing pass.
    let (final_text, original) = if cfg.postprocess_enabled {
        *state.lock().unwrap() = DaemonState::PostProcessing;
        let _ = TranscriberInterface::state_changed(&emitter, "PostProcessing").await;
        match crate::api::postprocess(
            &client,
            cfg.active_postprocess_url(),
            cfg.active_postprocess_key(),
            cfg.active_postprocess_model(),
            &cfg.postprocess_prompt,
            &raw,
        )
        .await
        {
            Ok(polished) => {
                tracing::info!("post-processing done: {} chars", polished.len());
                (polished, Some(raw))
            }
            Err(e) => {
                tracing::warn!("post-processing failed, falling back to raw: {e}");
                (raw, None)
            }
        }
    } else {
        (raw, None)
    };

    if cfg.save_history {
        let text_clone     = final_text.clone();
        let original_clone = original.clone();
        let wav_path_copy  = wav_path.clone();
        let db2            = db.clone();
        let _ = tokio::task::spawn_blocking(move || {
            db2.insert(&text_clone, original_clone.as_deref(), wav_path_copy.as_deref())
        })
        .await;
    }

    let _ = TranscriberInterface::transcription_ready(&emitter, &final_text).await;
    if cfg.audio_cues_enabled {
        crate::audio_cues::play_done();
    }
    *state.lock().unwrap() = DaemonState::Done;
    let _ = TranscriberInterface::state_changed(&emitter, "Done").await;
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
