//! Phase 1 de-risk spike: load Parakeet TDT v3 (multilingual), transcribe
//! assets/test.wav, print the text. Provider selected via PW_CUDA=1 (else CPU).
//! Model dir via PW_MODEL_DIR (default ./models/parakeet-v3).
use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode, Transcriber};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let model_dir = PathBuf::from(
        std::env::var("PW_MODEL_DIR").unwrap_or_else(|_| "./models/parakeet-v3".into()),
    );
    let want_cuda = std::env::var("PW_CUDA").map(|v| v == "1").unwrap_or(false);
    println!("model dir: {}", model_dir.display());

    // Load WAV (16-bit PCM or f32).
    let mut reader = hound::WavReader::open("assets/test.wav")?;
    let spec = reader.spec();
    let audio: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    println!(
        "loaded {} samples @ {} Hz, {} ch",
        audio.len(),
        spec.sample_rate,
        spec.channels
    );

    let provider = if want_cuda {
        ExecutionProvider::Cuda
    } else {
        ExecutionProvider::Cpu
    };
    println!("loading ParakeetTDT with provider {:?} ...", provider);
    let config = ExecutionConfig::new().with_execution_provider(provider);
    let mut model = ParakeetTDT::from_pretrained(&model_dir, Some(config))?;

    let result = model.transcribe_samples(
        audio,
        spec.sample_rate,
        spec.channels,
        Some(TimestampMode::Sentences),
    )?;
    println!("TEXT: {}", result.text);
    Ok(())
}
