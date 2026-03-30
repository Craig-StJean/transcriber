use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const SAMPLE_RATE: u32 = 44100;
const VOLUME:      f32 = 0.25;

/// Synthesize and play a sequence of tones on the default output device.
/// Each entry is `(frequency_hz, duration_ms)`. Runs on a new OS thread so
/// it never blocks the async runtime.
fn play_tones(freqs: &[(f32, u64)]) {
    let freqs = freqs.to_vec();
    std::thread::spawn(move || {
        let host   = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None    => {
                tracing::warn!("audio cues: no default output device");
                return;
            }
        };

        let stream_config = cpal::StreamConfig {
            channels:    2,
            sample_rate: SAMPLE_RATE,
            buffer_size: cpal::BufferSize::Default,
        };

        for (freq, dur_ms) in freqs {
            let total_samples = (SAMPLE_RATE as f32 * dur_ms as f32 / 1000.0) as usize;
            let fade_start    = (total_samples as f32 * 0.8) as usize;
            let fade_len      = total_samples - fade_start;

            // Use an Arc<Mutex<usize>> so the closure can track position.
            let pos = std::sync::Arc::new(std::sync::Mutex::new(0usize));
            let pos_cb = pos.clone();

            let stream = device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| {
                    let mut p = pos_cb.lock().unwrap();
                    for frame in data.chunks_mut(2) {
                        let val = if *p < total_samples {
                            let envelope = if *p >= fade_start {
                                1.0 - (*p - fade_start) as f32 / fade_len as f32
                            } else {
                                1.0
                            };
                            let t = *p as f32 / SAMPLE_RATE as f32;
                            (2.0 * std::f32::consts::PI * freq * t).sin() * VOLUME * envelope
                        } else {
                            0.0
                        };
                        *p += 1;
                        for s in frame.iter_mut() { *s = val; }
                    }
                },
                |e| tracing::warn!("audio cue stream error: {e}"),
                None,
            );

            match stream {
                Ok(s) => {
                    if let Err(e) = s.play() {
                        tracing::warn!("audio cue play error: {e}");
                        continue;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(dur_ms));
                    // `s` drops here, stopping the stream cleanly
                }
                Err(e) => tracing::warn!("audio cue build_output_stream error: {e}"),
            }
        }
    });
}

/// Ascending chirp played when recording starts.
pub fn play_start() {
    play_tones(&[(880.0, 80), (1320.0, 80)]);
}

/// Descending chirp played when transcription completes successfully.
pub fn play_done() {
    play_tones(&[(1320.0, 80), (880.0, 80)]);
}
