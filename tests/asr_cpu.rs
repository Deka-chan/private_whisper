//! Ignored integration test: transcribe the sample WAV on CPU. Needs the model
//! downloaded locally and ORT_DYLIB_PATH pointing at an onnxruntime dylib.
//! Run: PW_MODEL_DIR=./models/parakeet-v3 ORT_DYLIB_PATH=... cargo test --test asr_cpu -- --ignored
#[test]
#[ignore]
fn transcribes_sample_wav_non_empty() {
    let model_dir = std::path::PathBuf::from(
        std::env::var("PW_MODEL_DIR").unwrap_or_else(|_| "./models/parakeet-v3".into()),
    );
    let mut reader = hound::WavReader::open("assets/test.wav").unwrap();
    let spec = reader.spec();
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32768.0)
        .collect();
    let mut asr =
        privatewhisper::asr::Asr::load(&model_dir, privatewhisper::config::Provider::Cpu).unwrap();
    let text = asr.transcribe(samples, spec.sample_rate).unwrap();
    assert!(!text.trim().is_empty(), "expected non-empty transcription");
}
