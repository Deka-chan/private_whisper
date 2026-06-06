# privatewhisper Dictation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. In this project the implementer is **`codex exec`**: Claude dispatches each task to codex, then verifies the diff, runs tests, and commits.

**Goal:** A lightweight native Rust tray app for Windows where a global hotkey (toggle) records the mic, transcribes locally with Parakeet TDT 0.6b v3 (fp16) on the RTX 5060 via parakeet-rs (CUDA EP, CPU fallback), and auto-pastes the text at the cursor; the ~1.25 GB model downloads from HuggingFace on first run.

**Architecture:** One binary with a `tao` event loop driving a tray icon and a global hotkey; heavy work (model load, download, transcription) runs on a worker thread. Logic is split into focused modules (`config`, `model`, `audio`, `asr`, `inject`, `app`, `tray`) with a pure state machine at the core so most behavior is unit-testable without a GPU or mic. Platform/IO glue is thin and validated manually on Windows.

**Tech Stack:** Rust; `parakeet-rs` (ONNX Runtime / `ort`, CUDA feature) for ASR; `tao` + `tray-icon` + `global-hotkey` for UI/hotkey; `cpal` for audio; `arboard` + `enigo` for clipboard/paste; `reqwest` (blocking) for model download; `serde`/`toml` for config; `directories` for paths; `anyhow`/`thiserror` for errors; `hound` for test WAVs.

> **Reading note for the implementer:** Tasks are ordered to de-risk first. The pure-logic modules (Config, State machine, Audio conversion, Model manifest, Text normalization) have complete, exact code and strict TDD. The external-crate glue (ASR, download, paste, tray, hotkey, event loop) gives concrete code that the **Phase 1 spike confirms** — where a signature is marked "confirm via spike", check it against the crate version pinned in Phase 0 and adjust the call site only (the module boundary/types stay as written). Crate versions in Phase 0 are best-known as of 2026-06; pin exact versions during scaffold and update call sites if an API differs.

---

> **UPDATE 2026-06-07 — Phase 0 & 1 done; GPU approach finalized.** Scaffold built; the de-risk spike ran on the real RTX 5060. See **spec §14** for authoritative results. Key changes that override parts of this plan:
> - **Engine API:** model type is `ParakeetTDT` (not `Parakeet`); `transcribe_samples(audio: Vec<f32>, rate, channels, Some(TimestampMode::Sentences))`, trait `parakeet_rs::Transcriber`.
> - **Confirmed fp16 manifest:** `encoder-model.onnx` (grikdotnet `encoder-model.fp16.onnx`, 1.24 GB), `decoder_joint-model.onnx` (grikdotnet, 36 MB), `vocab.txt` (istupakov). No preprocessor/config.json.
> - **GPU on Blackwell requires `load-dynamic`** + Microsoft onnxruntime-gpu 1.24.x CUDA-13 (Azure `onnxruntime-cuda-13` feed) + CUDA-13 runtime/cuDNN9. Default ort onnxruntime lacks sm_120 (`cudaErrorNoKernelImageForDevice`). Cargo.toml now uses `parakeet-rs = { default-features=false, features=["cpu","cuda","load-dynamic"] }`.
> - **Phase 5 (model) expands to a `runtime` provisioner:** on first run download the model (1.24 GB) AND the right onnxruntime runtime — GPU (~2 GB) when a Blackwell NVIDIA GPU is present, else CPU onnxruntime (~15 MB) — then set `ORT_DYLIB_PATH`. **Phase 6 (asr)** uses load-dynamic (no static onnxruntime).
> - Phases 2–4 below (Config, State machine, Audio conversion) are unaffected and implemented as written.

## File Structure

```
privatewhisper/
├─ Cargo.toml                 # deps, fp16/cuda features, [[example]] spike
├─ src/
│  ├─ main.rs                 # entry: wire config→model→asr→event loop; worker thread
│  ├─ config.rs               # Config struct, Provider enum, load/save TOML, paths
│  ├─ model.rs                # ModelFile manifest, missing_files/is_complete, download+progress
│  ├─ audio.rs                # sample conversion (pure) + cpal Recorder wrapper
│  ├─ asr.rs                  # Asr: load(provider) CUDA→CPU fallback, transcribe()
│  ├─ inject.rs              # text normalize (pure) + paste() clipboard+Ctrl+V w/ restore
│  ├─ app.rs                  # State/Event/Action enums + transition() pure state machine
│  └─ tray.rs                 # tray menu + icon state helpers
├─ examples/
│  └─ spike.rs                # Phase 1 de-risk: load fp16 model, EP fallback, transcribe WAV
├─ tests/
│  └─ asr_cpu.rs              # #[ignore] integration: sample WAV → non-empty text on CPU EP
└─ assets/
   └─ test.wav                # short 16k mono sample for tests/spike (committed)
```

**Boundaries:** `app::transition` is pure (no IO) and is the heart of the UX. `config`, `model` (presence logic), `audio` (conversion), `inject` (normalize) expose pure functions that are unit-tested. `asr`, `model` (download), `inject` (paste), `tray`, `main` are thin IO/platform layers validated by smoke tests + the Windows manual checklist.

---

## Phase 0 — Scaffold & test harness

### Task 0.1: Create the Rust project and dependency manifest

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `assets/test.wav` (download a short 16 kHz mono sample; see step)

- [ ] **Step 1: Initialize the crate**

Run:
```bash
cd /home/lenovo/projects/superwhisper
cargo init --name privatewhisper
```
Expected: creates `Cargo.toml` and `src/main.rs`. (`Cargo.lock` is gitignored already.)

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "privatewhisper"
version = "0.1.0"
edition = "2021"

[dependencies]
parakeet-rs = { version = "0.3", features = ["cuda"] }
tao = "0.30"
tray-icon = "0.19"
global-hotkey = "0.6"
cpal = "0.15"
arboard = "3"
enigo = "0.2"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
directories = "5"
anyhow = "1"
thiserror = "2"
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
hound = "3"
tempfile = "3"

[[example]]
name = "spike"
```

Note: versions are best-known as of 2026-06. If `cargo build` reports a newer/older major, pin the latest compatible and adjust only the affected call sites. `parakeet-rs`'s `cuda` feature pulls a CUDA-enabled `ort`; on the Linux dev box without CUDA libs the build may still succeed but GPU init fails at runtime (that is expected — we use CPU there).

- [ ] **Step 3: Minimal `src/main.rs` placeholder**

```rust
fn main() {
    env_logger::init();
    println!("privatewhisper starting");
}
```

- [ ] **Step 4: Add a committed test sample WAV**

The repo needs a short (~2-4 s) 16 kHz mono WAV with clear speech for the spike and integration test. Use the sample shipped in parakeet/sherpa test data or record one.
Run (option A — reuse a known public sample):
```bash
mkdir -p assets
curl -L -o assets/test.wav https://github.com/k2-fsa/sherpa-onnx/raw/master/scripts/test_wavs/0.wav
```
If that URL 404s, record any short English+Russian utterance as 16 kHz mono PCM WAV and save to `assets/test.wav`. Verify:
```bash
python3 -c "import wave; w=wave.open('assets/test.wav'); print(w.getframerate(), w.getnchannels(), w.getnframes())"
```
Expected: framerate `16000`, channels `1`.

- [ ] **Step 5: Build to confirm deps resolve**

Run: `cargo build 2>&1 | tail -20`
Expected: compiles (first build downloads ONNX Runtime binaries; may take minutes). If a crate version fails to resolve, pin to the latest compatible and rebuild.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/main.rs assets/test.wav
git commit -m "chore: scaffold privatewhisper crate with deps and test sample"
```

---

## Phase 1 — De-risk spike (engine + model + GPU) ⚠️ GATE

This validates spec risks #1 (Blackwell/sm_120 + ONNX Runtime CUDA), #2 (exact model file manifest), #3 (parakeet-rs maturity) **before** building the app. The spike is a throwaway example. It must run on the **Windows + RTX 5060** machine for the CUDA verdict; on Linux it confirms the model + API on CPU.

### Task 1.1: Confirm the exact model manifest from parakeet-rs

**Files:** none (research task; results feed Task 1.2 and Phase 5).

- [ ] **Step 1: Read the parakeet-rs README/example for the v3 model**

Run:
```bash
cargo doc -p parakeet-rs --no-deps 2>/dev/null; echo "see https://github.com/altunenes/parakeet-rs"
```
Determine, from the crate's own docs/example, for the **v3 multilingual fp16** model:
- the exact HuggingFace repo(s) and filenames it loads (expected: `encoder-model.fp16.onnx`, `decoder_joint-model.fp16.onnx`, `vocab.txt`, plus any feature-extractor/preprocessor file such as `nemo128.onnx` and `config.json`),
- the `from_pretrained` directory layout it expects,
- the transcription call signature (expected: `transcribe_samples(samples: &[f32], sample_rate, timestamp_mode)` returning a struct with `.text`),
- the `ExecutionConfig` / `ExecutionProvider` API for selecting CUDA vs CPU.

- [ ] **Step 2: Record findings in the spec's "to confirm" section**

Append the confirmed filenames + URLs + sizes to `docs/superpowers/specs/2026-06-06-superwhisper-dictation-design.md` section 12, and commit:
```bash
git add docs/superpowers/specs/2026-06-06-superwhisper-dictation-design.md
git commit -m "docs: record confirmed parakeet-rs v3 fp16 model manifest"
```

### Task 1.2: Write and run the spike

**Files:**
- Create: `examples/spike.rs`

- [ ] **Step 1: Write `examples/spike.rs`**

This mirrors the eventual `asr.rs` so the learnings transfer directly. Adjust the parakeet-rs calls to the exact API confirmed in Task 1.1.

```rust
//! Throwaway de-risk spike: download/load Parakeet v3 fp16, try CUDA then CPU,
//! transcribe assets/test.wav, print the text and which provider was used.
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // 1) Model directory (manually populate for the spike, or let from_pretrained fetch).
    let model_dir = PathBuf::from(
        std::env::var("PW_MODEL_DIR").unwrap_or_else(|_| "./models/parakeet-v3-fp16".into()),
    );
    println!("model dir: {}", model_dir.display());

    // 2) Load the wav (16k mono expected; parakeet preprocessor resamples otherwise).
    let mut reader = hound::WavReader::open("assets/test.wav")?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    println!("loaded {} samples @ {} Hz", samples.len(), spec.sample_rate);

    // 3) Try CUDA, fall back to CPU. (Confirm exact API names via Task 1.1.)
    use parakeet_rs::{ExecutionConfig, ExecutionProvider, Parakeet};
    let (model, used) = match Parakeet::from_pretrained(
        &model_dir,
        Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda)),
    ) {
        Ok(m) => (m, "CUDA"),
        Err(e) => {
            eprintln!("CUDA init failed ({e}); falling back to CPU");
            (
                Parakeet::from_pretrained(
                    &model_dir,
                    Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cpu)),
                )?,
                "CPU",
            )
        }
    };

    // 4) Transcribe (confirm exact method name/signature via Task 1.1).
    let result = model.transcribe_samples(&samples, spec.sample_rate, None)?;
    println!("[{used}] text: {}", result.text);
    Ok(())
}
```

- [ ] **Step 2: Run the spike on Linux (CPU verdict + API/model sanity)**

Populate `./models/parakeet-v3-fp16` with the files confirmed in Task 1.1 (manual `curl` from HF, or rely on `from_pretrained` auto-fetch). Then:
```bash
cargo run --example spike 2>&1 | tail -30
```
Expected: prints loaded sample count and a non-empty transcription with `[CPU]` (or `[CUDA]` if WSL2 GPU is wired). If the API signature differs, fix the call to match the crate and re-run.

- [ ] **Step 3: Run the spike on Windows + RTX 5060 (the GPU GATE)**

On the Windows machine, build with the CUDA feature and run the same spike. Expected: `[CUDA] text: ...` non-empty, and `nvidia-smi` shows the process using the GPU during the run.

**GATE outcome:**
- ✅ CUDA works → proceed; the spike's load/transcribe code becomes `asr.rs` in Phase 6.
- ❌ `no kernel image` / sm_120 error → record the failing ONNX Runtime/CUDA version, try the ORT version that ships CUDA 12.6+/Blackwell support (update `ort`/`parakeet-rs` pin), re-run. If unresolved, the app still ships with CPU fallback; note this and continue (CUDA can be revisited).

- [ ] **Step 4: Commit the spike**

```bash
git add examples/spike.rs Cargo.toml
git commit -m "spike: validate parakeet-rs v3 fp16 load + EP fallback + transcribe"
```

---

## Phase 2 — Config module (pure, TDD)

### Task 2.1: Config struct, defaults, and round-trip serialization

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add `mod config;`)
- Test: in `src/config.rs` (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing tests**

In `src/config.rs`:
```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Cuda,
    Cpu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub hotkey: String,
    pub execution_provider: Provider,
    pub input_device: Option<String>,
    pub paste_delay_ms: u64,
    pub model_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Space".to_string(),
            execution_provider: Provider::Cuda,
            input_device: None,
            paste_delay_ms: 80,
            model_dir: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_ctrl_space_and_cuda() {
        let c = Config::default();
        assert_eq!(c.hotkey, "Ctrl+Space");
        assert_eq!(c.execution_provider, Provider::Cuda);
        assert_eq!(c.paste_delay_ms, 80);
    }

    #[test]
    fn toml_round_trip_preserves_values() {
        let c = Config {
            hotkey: "Ctrl+Alt+D".into(),
            execution_provider: Provider::Cpu,
            input_device: Some("Mic".into()),
            paste_delay_ms: 120,
            model_dir: None,
        };
        let s = toml::to_string(&c).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let back: Config = toml::from_str("hotkey = \"Ctrl+Space\"\n").unwrap();
        assert_eq!(back.execution_provider, Provider::Cuda); // from #[serde(default)]
        assert_eq!(back.paste_delay_ms, 80);
    }
}
```
Add `mod config;` to `src/main.rs`.

- [ ] **Step 2: Run tests to verify they pass (struct+derives are the impl here)**

Run: `cargo test config:: 2>&1 | tail -20`
Expected: 3 tests pass. (This task's "implementation" is the type + derives written alongside the tests; if any fail, fix the struct/attrs.)

- [ ] **Step 3: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat(config): Config struct with defaults and TOML round-trip"
```

### Task 2.2: Config paths, load, and save

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/config.rs` tests module:
```rust
    #[test]
    fn save_then_load_from_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let c = Config { paste_delay_ms: 200, ..Default::default() };
        c.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.paste_delay_ms, 200);
    }

    #[test]
    fn load_from_missing_path_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.toml");
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded, Config::default());
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test config::tests::save_then_load_from_explicit_path 2>&1 | tail -10`
Expected: FAIL — `no method named save_to`.

- [ ] **Step 3: Implement paths + load/save**

Append to `src/config.rs` (outside tests):
```rust
use std::path::Path;
use directories::ProjectDirs;

impl Config {
    /// Standard config file location: %APPDATA%/privatewhisper/config.toml (or platform equiv).
    pub fn default_path() -> anyhow::Result<PathBuf> {
        let pd = ProjectDirs::from("", "", "privatewhisper")
            .ok_or_else(|| anyhow::anyhow!("cannot determine config dir"))?;
        Ok(pd.config_dir().join("config.toml"))
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(toml::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Load from the default path, creating it with defaults if absent.
    pub fn load_or_create() -> anyhow::Result<Config> {
        let path = Self::default_path()?;
        let cfg = Self::load_from(&path)?;
        if !path.exists() {
            cfg.save_to(&path)?;
        }
        Ok(cfg)
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test config:: 2>&1 | tail -20`
Expected: all config tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): load/save with platform paths and default fallback"
```

---

## Phase 3 — App state machine (pure, TDD)

### Task 3.1: State/Event/Action and the toggle transition

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs` (add `mod app;`)

- [ ] **Step 1: Write the failing tests**

In `src/app.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    HotkeyPressed,
    TranscriptionDone,
    TranscriptionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    StartRecording,
    StopAndTranscribe,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_hotkey_starts_recording() {
        assert_eq!(transition(State::Idle, Event::HotkeyPressed),
                   (State::Recording, Action::StartRecording));
    }

    #[test]
    fn recording_hotkey_stops_and_transcribes() {
        assert_eq!(transition(State::Recording, Event::HotkeyPressed),
                   (State::Transcribing, Action::StopAndTranscribe));
    }

    #[test]
    fn transcribing_done_returns_to_idle() {
        assert_eq!(transition(State::Transcribing, Event::TranscriptionDone),
                   (State::Idle, Action::None));
    }

    #[test]
    fn transcribing_failed_returns_to_idle() {
        assert_eq!(transition(State::Transcribing, Event::TranscriptionFailed),
                   (State::Idle, Action::None));
    }

    #[test]
    fn hotkey_ignored_while_transcribing() {
        assert_eq!(transition(State::Transcribing, Event::HotkeyPressed),
                   (State::Transcribing, Action::None));
    }

    #[test]
    fn stray_completion_events_are_noops() {
        assert_eq!(transition(State::Idle, Event::TranscriptionDone),
                   (State::Idle, Action::None));
        assert_eq!(transition(State::Recording, Event::TranscriptionFailed),
                   (State::Recording, Action::None));
    }
}
```
Add `mod app;` to `src/main.rs`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test app:: 2>&1 | tail -10`
Expected: FAIL — `cannot find function transition`.

- [ ] **Step 3: Implement `transition`**

Append to `src/app.rs` (above tests):
```rust
/// Pure toggle state machine. No IO — the caller performs the returned Action.
pub fn transition(state: State, event: Event) -> (State, Action) {
    use Action::*;
    use Event::*;
    use State::*;
    match (state, event) {
        (Idle, HotkeyPressed) => (Recording, StartRecording),
        (Recording, HotkeyPressed) => (Transcribing, StopAndTranscribe),
        (Transcribing, TranscriptionDone) => (Idle, None),
        (Transcribing, TranscriptionFailed) => (Idle, None),
        // Everything else is a safe no-op that preserves the current state.
        (s, _) => (s, None),
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test app:: 2>&1 | tail -10`
Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat(app): pure toggle state machine (Idle/Recording/Transcribing)"
```

---

## Phase 4 — Audio (conversion pure-TDD; recorder wrapper)

### Task 4.1: Sample conversion helpers (pure, TDD)

**Files:**
- Create: `src/audio.rs`
- Modify: `src/main.rs` (add `mod audio;`)

- [ ] **Step 1: Write the failing tests**

In `src/audio.rs`:
```rust
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
```
Add `mod audio;` to `src/main.rs`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test audio:: 2>&1 | tail -10`
Expected: FAIL — `cannot find function i16_to_f32`.

- [ ] **Step 3: Implement the helpers**

Prepend to `src/audio.rs`:
```rust
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
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test audio:: 2>&1 | tail -10`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/audio.rs src/main.rs
git commit -m "feat(audio): i16->f32 and channel down-mix helpers"
```

### Task 4.2: cpal Recorder wrapper (IO; smoke-built)

**Files:**
- Modify: `src/audio.rs`

- [ ] **Step 1: Implement the Recorder**

This is IO glue (no unit test — validated by build + manual mic check on Windows). Append to `src/audio.rs`:
```rust
use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Captures the default input device into a shared buffer until stopped.
/// Stores the native sample rate and channel count; conversion to mono f32
/// happens in `stop()`.
pub struct Recorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

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
        let buf = buffer.clone();

        // Assume f32 input config (cpal's common default). For i16/u16 configs,
        // build the matching stream and convert via i16_to_f32 before pushing.
        let err_fn = |e| log::error!("audio stream error: {e}");
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| buf.lock().unwrap().extend_from_slice(data),
            err_fn,
            None,
        )?;
        stream.play()?;
        Ok(Recorder { stream, buffer, sample_rate, channels })
    }

    /// Stop and return (mono f32 samples, sample_rate).
    pub fn stop(self) -> (Vec<f32>, u32) {
        drop(self.stream); // stops capture
        let interleaved = self.buffer.lock().unwrap().clone();
        (to_mono_f32(&interleaved, self.channels), self.sample_rate)
    }
}
```

- [ ] **Step 2: Build**

Run: `cargo build 2>&1 | tail -20`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src/audio.rs
git commit -m "feat(audio): cpal Recorder start/stop returning mono f32 + rate"
```

---

## Phase 5 — Model manifest & download

### Task 5.1: Manifest + presence logic (pure, TDD)

**Files:**
- Create: `src/model.rs`
- Modify: `src/main.rs` (add `mod model;`)

- [ ] **Step 1: Write the failing tests**

In `src/model.rs`:
```rust
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct ModelFile {
    pub filename: &'static str,
    pub url: &'static str,
    pub expected_size: u64,
}

/// Confirmed in Phase 1 / Task 1.1. Replace URLs+sizes with the exact values
/// the spike validated for the parakeet-rs v3 fp16 layout.
pub const MODEL_FILES: &[ModelFile] = &[
    ModelFile { filename: "encoder-model.fp16.onnx", url: "https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16/resolve/main/encoder-model.fp16.onnx", expected_size: 0 },
    ModelFile { filename: "decoder_joint-model.fp16.onnx", url: "https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16/resolve/main/decoder_joint-model.fp16.onnx", expected_size: 0 },
    ModelFile { filename: "vocab.txt", url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/vocab.txt", expected_size: 0 },
    // Add feature-extractor/config files exactly as confirmed in Task 1.1.
];

/// A file counts as present if it exists and (when expected_size > 0) matches size.
fn file_present(dir: &Path, f: &ModelFile) -> bool {
    let p = dir.join(f.filename);
    match std::fs::metadata(&p) {
        Ok(m) => f.expected_size == 0 || m.len() == f.expected_size,
        Err(_) => false,
    }
}

pub fn missing_files(dir: &Path) -> Vec<&'static str> {
    MODEL_FILES.iter().filter(|f| !file_present(dir, f)).map(|f| f.filename).collect()
}

pub fn is_complete(dir: &Path) -> bool {
    missing_files(dir).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str, bytes: usize) {
        std::fs::write(dir.join(name), vec![0u8; bytes]).unwrap();
    }

    #[test]
    fn empty_dir_is_incomplete_and_lists_all() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_complete(dir.path()));
        assert_eq!(missing_files(dir.path()).len(), MODEL_FILES.len());
    }

    #[test]
    fn all_present_is_complete() {
        let dir = tempfile::tempdir().unwrap();
        for f in MODEL_FILES {
            touch(dir.path(), f.filename, 1);
        }
        assert!(is_complete(dir.path()));
        assert!(missing_files(dir.path()).is_empty());
    }

    #[test]
    fn size_mismatch_counts_as_missing_when_expected_set() {
        let dir = tempfile::tempdir().unwrap();
        let f = ModelFile { filename: "x.bin", url: "", expected_size: 100 };
        touch(dir.path(), "x.bin", 10);
        assert!(!file_present(dir.path(), &f));
    }
}
```
Add `mod model;` to `src/main.rs`.

- [ ] **Step 2: Run to verify it fails / then passes**

Run: `cargo test model:: 2>&1 | tail -15`
Expected: compiles and 3 tests pass (logic is written with the tests; if `size_mismatch` can't see `file_present`, keep `file_present` non-`pub` but in-module so the test reaches it — it does, same module).

- [ ] **Step 3: Commit**

```bash
git add src/model.rs src/main.rs
git commit -m "feat(model): manifest + presence/size detection logic"
```

### Task 5.2: Streaming downloader with progress

**Files:**
- Modify: `src/model.rs`

- [ ] **Step 1: Implement the downloader (IO; manual-validated)**

Append to `src/model.rs`:
```rust
use std::io::{Read, Write};

/// Progress callback: (file_index, files_total, bytes_done_this_file, bytes_total_this_file).
pub type ProgressFn<'a> = dyn FnMut(usize, usize, u64, u64) + 'a;

/// Download every missing file into `dir`, reporting progress. Idempotent:
/// files already present (and size-matching) are skipped.
pub fn ensure(dir: &Path, progress: &mut ProgressFn) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let total = MODEL_FILES.len();
    for (i, f) in MODEL_FILES.iter().enumerate() {
        if file_present(dir, f) {
            continue;
        }
        let resp = reqwest::blocking::get(f.url)?.error_for_status()?;
        let len = resp.content_length().unwrap_or(f.expected_size);
        let tmp = dir.join(format!("{}.part", f.filename));
        let mut out = std::fs::File::create(&tmp)?;
        let mut reader = resp;
        let mut buf = [0u8; 1 << 16];
        let mut done: u64 = 0;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            done += n as u64;
            progress(i, total, done, len);
        }
        out.flush()?;
        drop(out);
        std::fs::rename(&tmp, dir.join(f.filename))?; // atomic-ish: only rename complete files
    }
    if !is_complete(dir) {
        anyhow::bail!("model incomplete after download: missing {:?}", missing_files(dir));
    }
    Ok(())
}
```

- [ ] **Step 2: Build**

Run: `cargo build 2>&1 | tail -20`
Expected: compiles.

- [ ] **Step 3: Manual download smoke (optional, large)**

On a machine with bandwidth:
```bash
PW_MODEL_DIR=./models/parakeet-v3-fp16 cargo run --example spike
```
(Once `ensure` is wired in `main`, first run downloads ~1.25 GB; for now the spike + manual `curl` cover this.)

- [ ] **Step 4: Commit**

```bash
git add src/model.rs
git commit -m "feat(model): streaming downloader with progress and atomic rename"
```

---

## Phase 6 — ASR module

### Task 6.1: Asr wrapper with CUDA→CPU fallback

**Files:**
- Create: `src/asr.rs`
- Modify: `src/main.rs` (add `mod asr;`)

- [ ] **Step 1: Implement `Asr` (port from the validated spike)**

Use the exact parakeet-rs API confirmed in Phase 1. In `src/asr.rs`:
```rust
use std::path::Path;
use crate::config::Provider;
use parakeet_rs::{ExecutionConfig, ExecutionProvider, Parakeet};

pub struct Asr {
    inner: Parakeet,
    pub provider: Provider,
}

impl Asr {
    /// Load the model, preferring the requested provider. If CUDA fails to
    /// initialize, fall back to CPU and report the effective provider.
    pub fn load(model_dir: &Path, want: Provider) -> anyhow::Result<Asr> {
        let try_load = |p: Provider| -> anyhow::Result<Parakeet> {
            let ep = match p {
                Provider::Cuda => ExecutionProvider::Cuda,
                Provider::Cpu => ExecutionProvider::Cpu,
            };
            Ok(Parakeet::from_pretrained(
                model_dir,
                Some(ExecutionConfig::new().with_execution_provider(ep)),
            )?)
        };
        match want {
            Provider::Cuda => match try_load(Provider::Cuda) {
                Ok(inner) => Ok(Asr { inner, provider: Provider::Cuda }),
                Err(e) => {
                    log::warn!("CUDA init failed ({e}); using CPU");
                    Ok(Asr { inner: try_load(Provider::Cpu)?, provider: Provider::Cpu })
                }
            },
            Provider::Cpu => Ok(Asr { inner: try_load(Provider::Cpu)?, provider: Provider::Cpu }),
        }
    }

    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> anyhow::Result<String> {
        let r = self.inner.transcribe_samples(samples, sample_rate, None)?;
        Ok(r.text)
    }
}
```
Add `mod asr;` to `src/main.rs`.

- [ ] **Step 2: Write the ignored integration test**

Create `tests/asr_cpu.rs`:
```rust
//! Integration test: transcribe the committed sample WAV on the CPU EP.
//! Ignored by default because it needs the model downloaded locally.
//! Run with: PW_MODEL_DIR=./models/parakeet-v3-fp16 cargo test --test asr_cpu -- --ignored
#[test]
#[ignore]
fn transcribes_sample_wav_non_empty() {
    let model_dir = std::path::PathBuf::from(
        std::env::var("PW_MODEL_DIR").unwrap_or_else(|_| "./models/parakeet-v3-fp16".into()),
    );
    let mut reader = hound::WavReader::open("assets/test.wav").unwrap();
    let spec = reader.spec();
    let samples: Vec<f32> = reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect();

    let asr = privatewhisper::asr::Asr::load(&model_dir, privatewhisper::config::Provider::Cpu).unwrap();
    let text = asr.transcribe(&samples, spec.sample_rate).unwrap();
    assert!(!text.trim().is_empty(), "expected non-empty transcription");
}
```
Note: for the integration test to import `privatewhisper::asr`, the crate must expose a library target. If it is binary-only, add `src/lib.rs` re-exporting the modules (`pub mod config; pub mod asr; ...`) and have `main.rs` use the lib. Do that in this step if needed.

- [ ] **Step 3: Build + run the ignored test where the model exists**

Run (Linux dev, after model present):
```bash
PW_MODEL_DIR=./models/parakeet-v3-fp16 cargo test --test asr_cpu -- --ignored --nocapture 2>&1 | tail -20
```
Expected: PASS with non-empty text. If the crate had to become a lib, ensure `cargo build` still produces the binary.

- [ ] **Step 4: Commit**

```bash
git add src/asr.rs tests/asr_cpu.rs src/main.rs src/lib.rs 2>/dev/null; git add -A
git commit -m "feat(asr): parakeet-rs wrapper with CUDA->CPU fallback + integration test"
```

---

## Phase 7 — Text injection (paste like Superwhisper)

### Task 7.1: Text normalization (pure, TDD)

**Files:**
- Create: `src/inject.rs`
- Modify: `src/main.rs` (add `mod inject;`)

- [ ] **Step 1: Write the failing tests**

In `src/inject.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize("  привет мир \n"), "привет мир");
    }

    #[test]
    fn empty_or_whitespace_becomes_empty() {
        assert_eq!(normalize("   \n\t"), "");
    }
}
```
Add `mod inject;` to `src/main.rs`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test inject:: 2>&1 | tail -10`
Expected: FAIL — `cannot find function normalize`.

- [ ] **Step 3: Implement**

Prepend to `src/inject.rs`:
```rust
/// Normalize raw ASR output before insertion. Currently trims edges; kept as a
/// seam for future punctuation/casing tweaks.
pub fn normalize(raw: &str) -> String {
    raw.trim().to_string()
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test inject:: 2>&1 | tail -10`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/inject.rs src/main.rs
git commit -m "feat(inject): text normalization with trim"
```

### Task 7.2: Clipboard paste with save/restore (IO; manual-validated)

**Files:**
- Modify: `src/inject.rs`

- [ ] **Step 1: Implement `paste`**

Append to `src/inject.rs`:
```rust
use std::{thread, time::Duration};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Superwhisper-style insertion: save the current clipboard, set our text,
/// send Ctrl+V, wait `delay_ms`, then restore the previous clipboard.
/// On any paste failure the text is left on the clipboard (caller notifies).
pub fn paste(text: &str, delay_ms: u64) -> anyhow::Result<()> {
    let text = normalize(text);
    if text.is_empty() {
        return Ok(());
    }
    let mut clip = Clipboard::new()?;
    let previous = clip.get_text().ok();

    clip.set_text(text.clone())?;

    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Control, Direction::Press)?;
    enigo.key(Key::Unicode('v'), Direction::Click)?;
    enigo.key(Key::Control, Direction::Release)?;

    thread::sleep(Duration::from_millis(delay_ms));

    if let Some(prev) = previous {
        let _ = clip.set_text(prev); // best-effort restore
    }
    Ok(())
}
```
Note: confirm `enigo` 0.2 key API (`Key::Control`, `Key::Unicode('v')`, `Direction`) against the pinned version; adjust key names only if needed. The save/restore + Ctrl+V structure stays.

- [ ] **Step 2: Build**

Run: `cargo build 2>&1 | tail -20`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src/inject.rs
git commit -m "feat(inject): clipboard paste with save/restore and Ctrl+V"
```

---

## Phase 8 — Tray, hotkey, and event-loop wiring

### Task 8.1: Tray menu + icon state helpers

**Files:**
- Create: `src/tray.rs`
- Modify: `src/main.rs` (add `mod tray;`)

- [ ] **Step 1: Implement tray construction (IO glue)**

In `src/tray.rs` build the tray icon and a menu with items: status (disabled label), Provider toggle, Re-download model, Open config, Quit. Expose menu item IDs so `main` can match clicks. Provide a helper to set the tooltip/icon per `State`.
```rust
use tray_icon::{menu::{Menu, MenuItem, PredefinedMenuItem}, TrayIcon, TrayIconBuilder};

pub struct Tray {
    pub icon: TrayIcon,
    pub quit_id: tray_icon::menu::MenuId,
    pub redownload_id: tray_icon::menu::MenuId,
    pub open_config_id: tray_icon::menu::MenuId,
}

pub fn build() -> anyhow::Result<Tray> {
    let menu = Menu::new();
    let redownload = MenuItem::new("Re-download model", true, None);
    let open_config = MenuItem::new("Open config", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&redownload)?;
    menu.append(&open_config)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit)?;
    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("privatewhisper — idle")
        .build()?;
    Ok(Tray { icon, quit_id: quit.id().clone(), redownload_id: redownload.id().clone(), open_config_id: open_config.id().clone() })
}

pub fn tooltip_for(state: crate::app::State) -> &'static str {
    match state {
        crate::app::State::Idle => "privatewhisper — idle",
        crate::app::State::Recording => "privatewhisper — recording…",
        crate::app::State::Transcribing => "privatewhisper — transcribing…",
    }
}
```
Note: tray-icon needs an embedded image for the icon; add a small `assets/icon.png` and load via `tray_icon::Icon::from_path`/`from_rgba`. Confirm exact builder API against the pinned version. Add `mod tray;` to `src/main.rs`.

- [ ] **Step 2: Build**

Run: `cargo build 2>&1 | tail -20`
Expected: compiles (on headless Linux the tray may warn at runtime; build must pass).

- [ ] **Step 3: Commit**

```bash
git add src/tray.rs src/main.rs assets/icon.png 2>/dev/null; git add -A
git commit -m "feat(tray): tray icon, menu items, and per-state tooltip"
```

### Task 8.2: Wire everything in `main.rs` (event loop + worker thread)

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement the orchestration**

Wire: load config → `model::ensure` (with progress to tray tooltip) → `Asr::load` (provider from config, fallback) → register global hotkey (parse `config.hotkey`) → `tao` event loop. On hotkey, call `app::transition`; perform the `Action`:
- `StartRecording` → `audio::Recorder::start` (store in app state).
- `StopAndTranscribe` → take samples, send to worker thread (channel) which calls `Asr::transcribe` then `inject::paste`; on completion the worker posts `TranscriptionDone/Failed` back via a `tao` user-event proxy to update the tray and run the state machine.

```rust
// Sketch — the implementer fills in concrete tao/global-hotkey wiring confirmed
// against pinned versions. Keep heavy work OFF the event-loop thread.
use std::sync::mpsc;

enum UiEvent { Transcribed(String), Failed(String) }

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cfg = privatewhisper::config::Config::load_or_create()?;
    let model_dir = cfg.model_dir.clone().unwrap_or_else(default_model_dir);

    // Ensure model (blocking, with simple stderr/tray progress).
    privatewhisper::model::ensure(&model_dir, &mut |i, n, done, total| {
        log::info!("model file {}/{}: {}/{} bytes", i + 1, n, done, total);
    })?;

    let asr = privatewhisper::asr::Asr::load(&model_dir, cfg.execution_provider)?;
    log::info!("ASR ready on {:?}", asr.provider);

    // Event loop, tray, hotkey, worker thread, state machine.
    // (Concrete tao/global-hotkey/tray-icon event wiring goes here, using
    //  app::transition for every hotkey press and routing Actions as above.)
    Ok(())
}

fn default_model_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("", "", "privatewhisper")
        .map(|pd| pd.data_local_dir().join("models/parakeet-v3-fp16"))
        .unwrap_or_else(|| std::path::PathBuf::from("./models/parakeet-v3-fp16"))
}
```

- [ ] **Step 2: Build**

Run: `cargo build 2>&1 | tail -30`
Expected: compiles.

- [ ] **Step 3: Run headless sanity on Linux**

Run: `cargo run 2>&1 | tail -20`
Expected: logs "ASR ready on Cpu" (CUDA likely unavailable on dev box) and stays alive (or exits cleanly if no display for tray). Errors about display/tray are acceptable on headless WSL2; the goal is that startup → model check → ASR load works.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire config, model, asr, hotkey, tray, and worker into main"
```

---

## Phase 9 — Windows end-to-end validation & packaging

### Task 9.1: Manual validation on Windows + RTX 5060

**Files:** none (manual checklist; record results in a commit message or `docs/`).

- [ ] **Step 1: Build release on Windows**

Run (Windows shell): `cargo build --release 2>&1 | tail -20`
Expected: produces `target/release/privatewhisper.exe`.

- [ ] **Step 2: First-run model download**

Delete any local model dir; run the exe. Expected: it downloads ~1.25 GB to `%LOCALAPPDATA%/privatewhisper/models/parakeet-v3-fp16/` with progress logged, then loads ASR.

- [ ] **Step 3: CUDA verdict**

Check logs say `ASR ready on Cuda`. Run `nvidia-smi` during a transcription — expected: the process appears using the GPU. If it says `Cpu`, apply the Phase 1 GATE remediation (ORT/CUDA version for sm_120).

- [ ] **Step 4: End-to-end dictation**

In Notepad and a browser text field: press `Ctrl+Space`, speak a Russian sentence, press `Ctrl+Space` again. Expected: the transcribed text is auto-pasted at the cursor within ~1 s; the prior clipboard contents are restored afterward. Repeat in English.

- [ ] **Step 5: Fallback + error paths**

Set `execution_provider = "cpu"` in config → restart → expect `ASR ready on Cpu` and working (slower) dictation. Unplug the mic → press hotkey → expect a graceful notification, no crash.

- [ ] **Step 6: Record results**

```bash
git commit --allow-empty -m "test: Windows+RTX 5060 e2e validated (CUDA, paste, fallback)"
```

### Task 9.2: Package a distributable zip

**Files:**
- Create: `scripts/package.ps1` (or document the manual steps)

- [ ] **Step 1: Assemble the zip**

Bundle `privatewhisper.exe` + the ONNX Runtime CUDA DLLs (and cuDNN, if required) that `ort` produced next to the binary, plus `assets/icon.png`. The model is NOT included (downloaded on first run). Document required NVIDIA driver / CUDA version learned from Phase 1.

- [ ] **Step 2: Smoke-test the zip on a clean Windows profile**

Unzip elsewhere, run, confirm first-run download + dictation work.

- [ ] **Step 3: Commit**

```bash
git add scripts/package.ps1
git commit -m "build: package script for Windows distributable zip"
```

---

## Self-Review (completed during authoring)

**Spec coverage check (spec → task):**
- Lightweight exe + model downloaded on first run → Phase 5 (manifest/download), Phase 9 (packaging excludes model). ✓
- parakeet-rs + fp16 + CUDA→CPU fallback → Phase 1 (spike), Phase 6 (`Asr::load`). ✓
- Toggle hotkey → Phase 3 (state machine), Phase 8 (hotkey wiring). ✓
- Superwhisper-style auto-paste with clipboard restore → Phase 7. ✓
- TOML config with `Ctrl+Space` default, provider, paste delay → Phase 2. ✓
- Tray with status/provider/redownload/quit → Phase 8.1. ✓
- Worker thread / responsive event loop → Phase 8.2. ✓
- Error handling (no mic, CUDA fail, download fail, empty result, paste fail) → fallback in `Asr::load` (Phase 6), `ensure` bail (Phase 5), empty-text guard in `paste` (Phase 7), manual paths Phase 9.5. ✓
- Testing: unit (config/state/audio/manifest/normalize) + ignored CPU integration + Windows manual checklist → Phases 2–7 + 9. ✓
- Risk #1 sm_120, #2 manifest, #3 maturity → Phase 1 GATE. ✓

**Placeholder scan:** External-crate call sites are marked "confirm via spike/pinned version" with concrete code given; module boundaries and pure logic are fully specified. No "TODO/handle edge cases" left as logic gaps.

**Type consistency:** `Provider` (config.rs) is the single provider type used by `Asr::load`. `State`/`Event`/`Action` (app.rs) match between `transition` and its callers. `ModelFile`/`missing_files`/`is_complete`/`ensure` names are consistent across model.rs and main.rs. `normalize`/`paste` consistent across inject.rs and the worker. `Recorder::start/stop` consistent across audio.rs and main.rs.

**Known intentional unknowns (resolved at execution):** exact parakeet-rs v3 fp16 filenames/URLs/sizes (Task 1.1 → fills `MODEL_FILES`), exact crate API signatures for parakeet-rs/enigo/tray-icon/global-hotkey/tao (pinned in Phase 0, confirmed in Phase 1, adjusted at call sites only).
