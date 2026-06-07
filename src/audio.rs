/// Convert signed 16-bit PCM to normalized f32 in [-1.0, 1.0].
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| s as f32 / 32768.0).collect()
}

/// Peak absolute amplitude of a block, clamped to [0.0, 1.0].
pub fn block_peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, &s| m.max(s.abs())).min(1.0)
}

/// Down-mix interleaved multi-channel f32 to mono by averaging channels.
pub fn to_mono_f32(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    if ch == 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(ch)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Resample mono f32 audio to 16 kHz — the rate Parakeet requires (it does not
/// resample internally and errors on any other rate).
///
/// Passthrough when already 16 kHz. For downsampling (the common mic case, e.g.
/// 48 kHz → 16 kHz) each output sample is the average of the source window it
/// covers, giving basic anti-aliasing; for upsampling it falls back to
/// nearest-source sampling.
pub fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    const TARGET: u32 = 16_000;
    if samples.is_empty() || from_rate == TARGET {
        return samples.to_vec();
    }
    let out_len = (samples.len() as u64 * TARGET as u64 / from_rate as u64) as usize;
    if out_len == 0 {
        return Vec::new();
    }
    let step = from_rate as f64 / TARGET as f64; // source samples per output sample
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let start = (i as f64 * step) as usize;
        let end = (((i + 1) as f64 * step).ceil() as usize)
            .min(samples.len())
            .max(start + 1);
        let window = &samples[start..end];
        out.push(window.iter().copied().sum::<f32>() / window.len() as f32);
    }
    out
}

#[cfg(target_os = "windows")]
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Captures the default input device into a shared buffer until stopped.
///
/// Stores the native sample rate and channel count; conversion to mono f32
/// happens in `stop()`.
#[cfg(target_os = "windows")]
pub struct Recorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

#[cfg(target_os = "windows")]
impl Recorder {
    /// Start capturing. `device_name` selects an input by name; None = default.
    pub fn start(device_name: Option<&str>, level: Arc<AtomicU32>) -> anyhow::Result<Recorder> {
        let host = cpal::default_host();
        let device = match device_name {
            Some(name) => host
                .input_devices()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| anyhow::anyhow!("input device '{name}' not found"))?,
            None => host
                .default_input_device()
                .ok_or_else(|| anyhow::anyhow!("no default input device"))?,
        };
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let stream_config = config.clone().into();
        let err_fn = |e| log::error!("audio stream error: {e}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => {
                build_input_stream::<i8, _>(&device, &stream_config, buffer.clone(), level, err_fn)?
            }
            cpal::SampleFormat::I16 => build_input_stream::<i16, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::I32 => build_input_stream::<i32, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::I64 => build_input_stream::<i64, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::U8 => {
                build_input_stream::<u8, _>(&device, &stream_config, buffer.clone(), level, err_fn)?
            }
            cpal::SampleFormat::U16 => build_input_stream::<u16, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::U32 => build_input_stream::<u32, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::U64 => build_input_stream::<u64, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::F32 => build_input_stream::<f32, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            cpal::SampleFormat::F64 => build_input_stream::<f64, _>(
                &device,
                &stream_config,
                buffer.clone(),
                level,
                err_fn,
            )?,
            sample_format => anyhow::bail!("unsupported input sample format '{sample_format}'"),
        };
        stream.play()?;

        Ok(Recorder {
            stream,
            buffer,
            sample_rate,
            channels,
        })
    }

    /// Stop and return (16 kHz mono f32 samples, 16000). Audio is down-mixed to
    /// mono and resampled to 16 kHz, which is the only rate Parakeet accepts.
    pub fn stop(self) -> (Vec<f32>, u32) {
        drop(self.stream);
        let interleaved = self.buffer.lock().unwrap().clone();
        let mono = to_mono_f32(&interleaved, self.channels);
        (resample_to_16k(&mono, self.sample_rate), 16_000)
    }
}

#[cfg(target_os = "windows")]
fn build_input_stream<T, E>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    err_fn: E,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
    E: FnMut(cpal::StreamError) + Send + 'static,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let converted: Vec<f32> = data
                .iter()
                .map(|&sample| sample.to_sample::<f32>())
                .collect();
            let peak = block_peak(&converted);
            level.store(peak.to_bits(), Ordering::Relaxed);
            buffer.lock().unwrap().extend(converted);
        },
        err_fn,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i16_converts_to_normalized_f32() {
        let out = i16_to_f32(&[0, 16384, -32768]);
        assert!((out[0] - 0.0).abs() < 1e-6);
        assert!((out[1] - 0.5).abs() < 1e-3);
        assert!((out[2] + 1.0).abs() < 1e-3);
    }

    #[test]
    fn block_peak_returns_peak_absolute_amplitude_clamped_to_unit_range() {
        assert_eq!(block_peak(&[0.0, -0.5, 0.25]), 0.5);
        assert_eq!(block_peak(&[]), 0.0);
        assert_eq!(block_peak(&[-1.25, 0.75]), 1.0);
    }

    #[test]
    fn stereo_downmixes_to_mono_by_averaging() {
        // interleaved L,R,L,R
        let out = to_mono_f32(&[0.0, 1.0, 0.5, -0.5], 2);
        assert_eq!(out, vec![0.5, 0.0]);
    }

    #[test]
    fn mono_passthrough() {
        let out = to_mono_f32(&[0.1, 0.2, 0.3], 1);
        assert_eq!(out, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn resample_passthrough_at_16k() {
        let s = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_to_16k(&s, 16_000), s);
    }

    #[test]
    fn resample_empty_is_empty() {
        assert!(resample_to_16k(&[], 48_000).is_empty());
    }

    #[test]
    fn resample_48k_to_16k_thirds_length_and_preserves_dc() {
        // constant 0.5 signal: 9 samples @ 48k -> 3 samples @ 16k, still 0.5
        let s = vec![0.5f32; 9];
        let out = resample_to_16k(&s, 48_000);
        assert_eq!(out.len(), 3);
        for v in out {
            assert!((v - 0.5).abs() < 1e-6);
        }
    }
}
