use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

/// Shared in-memory buffer of raw f32 audio samples.
pub type PcmBuffer = Arc<Mutex<Vec<f32>>>;

/// Start capturing from the default input device at `sample_rate` Hz (mono).
///
/// Returns `(buffer, stop_tx)`.  Send `true` on `stop_tx` — or simply drop it —
/// to stop the stream and let the background thread exit.
pub fn start_capture(sample_rate: u32) -> Result<(PcmBuffer, watch::Sender<bool>)> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no default audio input device found"))?;

    tracing::info!(
        "capturing from device: {}",
        device.description().map(|d| d.name().to_string()).unwrap_or_else(|_| "unknown".into())
    );

    let buffer: PcmBuffer = Arc::new(Mutex::new(Vec::with_capacity(
        // Pre-allocate for ~60 seconds to avoid reallocs mid-recording
        sample_rate as usize * 60,
    )));

    let stream = build_stream(&device, sample_rate, buffer.clone())?;
    stream.play()?;

    let (stop_tx, mut stop_rx) = watch::channel(false);

    // Hold the stream alive on a dedicated blocking thread until stop is signalled.
    // A dropped sender counts as a stop too: every exit path in dbus.rs is
    // supposed to send `true`, but if one ever forgets, the mic must not stay
    // open (and this thread must not spin) for the life of the daemon.
    tokio::task::spawn_blocking(move || {
        let _stream = stream; // keep alive
        loop {
            match stop_rx.has_changed() {
                Err(_) => break, // sender dropped
                Ok(_) if *stop_rx.borrow_and_update() => break,
                Ok(_) => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        tracing::debug!("audio capture thread exiting");
    });

    Ok((buffer, stop_tx))
}

/// Open an input stream that appends mono f32 samples at `sample_rate` to `buf`.
///
/// Asks for exactly that first, which PipeWire/PulseAudio and ALSA's `default`
/// plug device convert for us. Raw `hw:` ALSA devices do no conversion and
/// reject it, so fall back to the device's own default config and downmix /
/// resample here instead.
fn build_stream(device: &cpal::Device, sample_rate: u32, buf: PcmBuffer) -> Result<cpal::Stream> {
    let wanted = cpal::StreamConfig {
        channels: 1,
        sample_rate,  // SampleRate is now type SampleRate = u32 in cpal 0.17
        buffer_size: cpal::BufferSize::Default,
    };
    let buf_write = buf.clone();
    match device.build_input_stream(
        &wanted,
        move |data: &[f32], _| {
            buf_write.lock().unwrap().extend_from_slice(data);
        },
        |err| tracing::error!("audio stream error: {err}"),
        None,
    ) {
        Ok(s) => return Ok(s),
        Err(e) => tracing::info!("device rejected mono {sample_rate} Hz f32 ({e}); using its default config"),
    }

    let default = device.default_input_config()?;
    let config = default.config();
    tracing::info!(
        "capturing at {} ch / {} Hz / {:?}, converting to mono {sample_rate} Hz",
        config.channels, config.sample_rate, default.sample_format()
    );
    match default.sample_format() {
        SampleFormat::F32 => build_converting::<f32>(device, &config, sample_rate, buf),
        SampleFormat::I16 => build_converting::<i16>(device, &config, sample_rate, buf),
        SampleFormat::I32 => build_converting::<i32>(device, &config, sample_rate, buf),
        SampleFormat::U16 => build_converting::<u16>(device, &config, sample_rate, buf),
        other => Err(anyhow!("unsupported input sample format {other:?}")),
    }
}

fn build_converting<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    target_rate: u32,
    buf: PcmBuffer,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels.max(1) as usize;
    let mut resampler = LinearResampler::new(config.sample_rate, target_rate);
    let mut mono = Vec::new();
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            mono.clear();
            mono.extend(data.chunks(channels).map(|frame| {
                frame.iter().map(|&s| f32::from_sample(s)).sum::<f32>() / frame.len() as f32
            }));
            resampler.process(&mono, &mut buf.lock().unwrap());
        },
        |err| tracing::error!("audio stream error: {err}"),
        None,
    )?;
    Ok(stream)
}

/// Streaming linear-interpolation resampler.
///
/// Speech models are trained on far worse than linear interpolation's
/// aliasing, so this trades fidelity for having no dependency. State carries
/// across callbacks so chunk boundaries don't click.
struct LinearResampler {
    /// Input samples advanced per output sample.
    step: f64,
    /// Read position, in input samples, relative to `prev` at index 0.
    pos:  f64,
    /// Last sample of the previous chunk.
    prev: f32,
}

impl LinearResampler {
    fn new(from: u32, to: u32) -> Self {
        Self { step: from as f64 / to.max(1) as f64, pos: 1.0, prev: 0.0 }
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }
        // Index 0 is the carried-over `prev`, 1..=len the new chunk.
        let prev = self.prev;
        let at = |i: usize| if i == 0 { prev } else { input[i - 1] };
        let len = input.len() as f64;
        while self.pos < len {
            let i = self.pos as usize;
            let frac = (self.pos - i as f64) as f32;
            out.push(at(i) * (1.0 - frac) + at(i + 1) * frac);
            self.pos += self.step;
        }
        self.pos -= len;
        self.prev = input[input.len() - 1];
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_halves_rate_across_chunks() {
        let mut r = LinearResampler::new(32_000, 16_000);
        let mut out = Vec::new();
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        for chunk in input.chunks(37) {
            r.process(chunk, &mut out);
        }
        assert_eq!(out.len(), 500);
        // Every output lands on an even input index, chunk boundaries included.
        for (k, s) in out.iter().enumerate() {
            assert!((s - (2 * k) as f32).abs() < 1e-3, "sample {k} = {s}");
        }
    }

    #[test]
    fn resampler_upsamples_by_interpolating() {
        let mut r = LinearResampler::new(8_000, 16_000);
        let mut out = Vec::new();
        r.process(&[0.0, 1.0, 2.0], &mut out);
        // The last input sample needs its successor before it can be emitted.
        assert_eq!(out, vec![0.0, 0.5, 1.0, 1.5]);
    }
}
