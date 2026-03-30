use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use zbus::{interface, object_server::SignalEmitter, Connection};

use crate::audio::{self, PcmBuffer};
use common::config::AppConfig;

// ── State machine ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonState {
    Idle,
    Recording,
    Transcribing,
    Done,
    Error(String),
}

impl DaemonState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle         => "Idle",
            Self::Recording    => "Recording",
            Self::Transcribing => "Transcribing",
            Self::Done         => "Done",
            Self::Error(_)     => "Error",
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
    pub http_client: reqwest::Client,
    pub db:          Arc<crate::db::Database>,
}

// ── zbus interface implementation ─────────────────────────────────────────────

#[interface(name = "org.transcriber.Daemon")]
impl TranscriberInterface {

    // ── Signals ──────────────────────────────────────────────────────────────

    /// Emitted whenever the daemon state changes.
    /// Possible values: "Idle" | "Recording" | "Transcribing" | "Done" | "Error"
    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;

    /// Emitted after a successful transcription with the resulting text.
    #[zbus(signal)]
    async fn transcription_ready(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()>;

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
            if *state == DaemonState::Recording || *state == DaemonState::Transcribing {
                return Err(zbus::fdo::Error::Failed(
                    "already recording or transcribing".into(),
                ));
            }
        }

        let sample_rate = self.config.lock().unwrap().sample_rate;
        let vad_enabled = self.config.lock().unwrap().vad_enabled;
        let (buf, stop_tx) =
            audio::start_capture(sample_rate).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        *self.audio_buf.lock().unwrap() = Some(buf.clone());
        *self.stop_tx.lock().unwrap() = Some(stop_tx);
        *self.state.lock().unwrap() = DaemonState::Recording;

        Self::state_changed(&emitter, "Recording").await?;

        if self.config.lock().unwrap().audio_cues_enabled {
            crate::audio_cues::play_start();
        }

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
                        (rms * 25.0).clamp(0.0, 1.0)
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
                                match do_transcription(client, cfg.clone(), samples, db).await {
                                    Ok(text) => {
                                        tracing::info!(
                                            "VAD transcription done: {} chars",
                                            text.len()
                                        );
                                        let _ = Self::transcription_ready(&emitter, &text).await;
                                        if cfg.audio_cues_enabled {
                                            crate::audio_cues::play_done();
                                        }
                                        *state.lock().unwrap() = DaemonState::Done;
                                        let _ = Self::state_changed(&emitter, "Done").await;
                                    }
                                    Err(e) => {
                                        tracing::error!("VAD transcription failed: {e}");
                                        *state.lock().unwrap() = DaemonState::Error(e.to_string());
                                        let _ = Self::state_changed(&emitter, "Error").await;
                                    }
                                }
                            });
                            break;
                        }
                    }
                }
            }
        });

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
            match do_transcription(client, cfg.clone(), samples, db).await {
                Ok(text) => {
                    tracing::info!("transcription done: {} chars", text.len());
                    let _ = Self::transcription_ready(&emitter, &text).await;
                    if cfg.audio_cues_enabled {
                        crate::audio_cues::play_done();
                    }
                    *state.lock().unwrap() = DaemonState::Done;
                    let _ = Self::state_changed(&emitter, "Done").await;
                }
                Err(e) => {
                    tracing::error!("transcription failed: {e}");
                    *state.lock().unwrap() = DaemonState::Error(e.to_string());
                    let _ = Self::state_changed(&emitter, "Error").await;
                }
            }
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
        *self.audio_buf.lock().unwrap() = None;
        *self.state.lock().unwrap() = DaemonState::Idle;
        Self::state_changed(&emitter, "Idle").await?;
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

async fn do_transcription(
    client: reqwest::Client,
    cfg: AppConfig,
    samples: Vec<f32>,
    db: Arc<crate::db::Database>,
) -> Result<String> {
    let wav = audio::encode_wav(&samples, cfg.sample_rate)?;

    // Optionally persist the recording to disk.
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

    match transcribe_with_retry(&client, &cfg, wav).await {
        Ok(text) => {
            if cfg.save_history {
                let text_clone    = text.clone();
                let wav_path_copy = wav_path.clone();
                tokio::task::spawn_blocking(move || {
                    db.insert(&text_clone, wav_path_copy.as_deref())
                })
                .await??;
            }
            Ok(text)
        }
        Err(e) => {
            if cfg.save_history {
                let err_str      = e.to_string();
                let wav_path_copy = wav_path.clone();
                tokio::task::spawn_blocking(move || {
                    db.insert_failed(&err_str, wav_path_copy.as_deref())
                })
                .await??;
            }
            Err(e)
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
