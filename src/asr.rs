use std::path::Path;

use crate::config::Provider;
use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode, Transcriber};

pub struct Asr {
    inner: ParakeetTDT,
    pub provider: Provider,
}

impl Asr {
    /// Load the model with the requested provider. If CUDA init fails, fall back to CPU.
    pub fn load(model_dir: &Path, want: Provider) -> anyhow::Result<Asr> {
        let build = |p: Provider| -> anyhow::Result<ParakeetTDT> {
            let ep = match p {
                Provider::Cuda => ExecutionProvider::Cuda,
                Provider::Cpu => ExecutionProvider::Cpu,
            };
            Ok(ParakeetTDT::from_pretrained(
                model_dir,
                Some(ExecutionConfig::new().with_execution_provider(ep)),
            )?)
        };

        match want {
            Provider::Cuda => match build(Provider::Cuda) {
                Ok(inner) => Ok(Asr {
                    inner,
                    provider: Provider::Cuda,
                }),
                Err(e) => {
                    log::warn!("CUDA init failed ({e}); using CPU");
                    Ok(Asr {
                        inner: build(Provider::Cpu)?,
                        provider: Provider::Cpu,
                    })
                }
            },
            Provider::Cpu => Ok(Asr {
                inner: build(Provider::Cpu)?,
                provider: Provider::Cpu,
            }),
        }
    }

    /// Transcribe mono f32 samples at the given sample rate.
    pub fn transcribe(&mut self, samples: Vec<f32>, sample_rate: u32) -> anyhow::Result<String> {
        let r = self.inner.transcribe_samples(
            samples,
            sample_rate,
            1,
            Some(TimestampMode::Sentences),
        )?;
        Ok(r.text)
    }
}
