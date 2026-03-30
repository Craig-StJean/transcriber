use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

/// Shared in-memory buffer of raw f32 audio samples.
pub type PcmBuffer = Arc<Mutex<Vec<f32>>>;

/// Start capturing from the default input device at `sample_rate` Hz (mono).
///
/// Returns `(buffer, stop_tx)`.  Send `true` on `stop_tx` to stop the stream
/// and allow the background thread to exit.
pub fn start_capture(sample_rate: u32) -> Result<(PcmBuffer, watch::Sender<bool>)> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no default audio input device found"))?;

    tracing::info!(
        "capturing from device: {}",
        device.description().map(|d| d.name().to_string()).unwrap_or_else(|_| "unknown".into())
    );

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate,  // SampleRate is now type SampleRate = u32 in cpal 0.17
        buffer_size: cpal::BufferSize::Default,
    };

    let buffer: PcmBuffer = Arc::new(Mutex::new(Vec::with_capacity(
        // Pre-allocate for ~60 seconds to avoid reallocs mid-recording
        sample_rate as usize * 60,
    )));
    let buf_write = buffer.clone();

    let (stop_tx, stop_rx) = watch::channel(false);

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            buf_write.lock().unwrap().extend_from_slice(data);
        },
        |err| tracing::error!("audio stream error: {err}"),
        None,
    )?;

    stream.play()?;

    // Hold the stream alive on a dedicated blocking thread until stop is signalled.
    tokio::task::spawn_blocking(move || {
        let _stream = stream; // keep alive
        let rx = stop_rx;
        loop {
            if *rx.borrow() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        tracing::debug!("audio capture thread exiting");
    });

    Ok((buffer, stop_tx))
}

/// Encode a slice of f32 PCM samples as a 16-bit mono WAV in memory.
///
/// The WAV bytes are returned as a `Vec<u8>` ready to POST directly to the
/// transcription API — no temp files are written to disk.
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
    let block_align: u16 = num_channels * bits_per_sample / 8;
    let data_size = (samples.len() * 2) as u32; // 2 bytes per i16 sample
    let riff_size = 36 + data_size; // everything after the first 8 RIFF bytes

    let mut buf = Cursor::new(Vec::with_capacity(44 + data_size as usize));

    // ── RIFF chunk ───────────────────────────────────────────────────────────
    buf.write_all(b"RIFF")?;
    buf.write_all(&riff_size.to_le_bytes())?;
    buf.write_all(b"WAVE")?;

    // ── fmt sub-chunk ────────────────────────────────────────────────────────
    buf.write_all(b"fmt ")?;
    buf.write_all(&16u32.to_le_bytes())?;         // sub-chunk size (PCM = 16)
    buf.write_all(&1u16.to_le_bytes())?;           // AudioFormat: PCM
    buf.write_all(&num_channels.to_le_bytes())?;
    buf.write_all(&sample_rate.to_le_bytes())?;
    buf.write_all(&byte_rate.to_le_bytes())?;
    buf.write_all(&block_align.to_le_bytes())?;
    buf.write_all(&bits_per_sample.to_le_bytes())?;

    // ── data sub-chunk ───────────────────────────────────────────────────────
    buf.write_all(b"data")?;
    buf.write_all(&data_size.to_le_bytes())?;

    for &s in samples {
        let i = (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        buf.write_all(&i.to_le_bytes())?;
    }

    Ok(buf.into_inner())
}
