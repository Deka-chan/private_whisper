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
