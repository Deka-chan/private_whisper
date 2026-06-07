/// Convert signed 16-bit PCM to normalized f32 in [-1.0, 1.0].
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| s as f32 / 32768.0).collect()
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

#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};

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
    pub fn start(device_name: Option<&str>) -> anyhow::Result<Recorder> {
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
                build_input_stream::<i8, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::I16 => {
                build_input_stream::<i16, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::I32 => {
                build_input_stream::<i32, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::I64 => {
                build_input_stream::<i64, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::U8 => {
                build_input_stream::<u8, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::U16 => {
                build_input_stream::<u16, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::U32 => {
                build_input_stream::<u32, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::U64 => {
                build_input_stream::<u64, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::F32 => {
                build_input_stream::<f32, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
            cpal::SampleFormat::F64 => {
                build_input_stream::<f64, _>(&device, &stream_config, buffer.clone(), err_fn)?
            }
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

    /// Stop and return (mono f32 samples, sample_rate).
    pub fn stop(self) -> (Vec<f32>, u32) {
        drop(self.stream);
        let interleaved = self.buffer.lock().unwrap().clone();
        (to_mono_f32(&interleaved, self.channels), self.sample_rate)
    }
}

#[cfg(target_os = "windows")]
fn build_input_stream<T, E>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
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
            buffer
                .lock()
                .unwrap()
                .extend(data.iter().map(|&sample| sample.to_sample::<f32>()));
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
}
