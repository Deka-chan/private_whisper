# privatewhisper

**Local, private voice-to-text for Windows.** Press a hotkey, speak, and your words are transcribed *on your own NVIDIA GPU* with [NVIDIA Parakeet TDT 0.6b v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) (multilingual) and pasted at the cursor — nothing leaves your machine.

A lightweight Rust app: a global hotkey, a tray icon, and a live waveform overlay. The ~25 MB executable downloads the model and the GPU runtime itself on first run.

---

## Features

- 🔒 **Fully local** — audio and text never leave your computer.
- 🎙️ **Toggle dictation** — `Ctrl+Space` to start, `Ctrl+Space` to stop → text is pasted at the cursor (clipboard + `Ctrl+V`, with the previous clipboard restored).
- 🌍 **Multilingual** — 25 languages with automatic language detection (Russian, English, German, Spanish, …).
- ⚡ **Runs on your GPU** — Parakeet TDT v3 (fp16) via ONNX Runtime CUDA, including **NVIDIA RTX 50-series (Blackwell, sm_120)**; falls back to CPU if the GPU can't start.
- 〰️ **Live overlay** — a small always-on-top waveform that scrolls while you speak (non-activating & click-through, so it never steals focus).
- 🪶 **Lightweight distribution** — a ~25 MB exe that fetches the model (~1.2 GB) and CUDA runtime (~1.2 GB) on first run.

## Requirements

- Windows 10/11 x64
- An **NVIDIA RTX 50-series (Blackwell)** GPU + a recent driver (CUDA 13 capable). Other CUDA GPUs / CPU may work but are untested.
- [Microsoft Visual C++ Redistributable 2015–2022 (x64)](https://aka.ms/vs/17/release/vc_redist.x64.exe) — required by the ONNX Runtime DLLs.

## Install & run

1. Download the latest release and unzip it to a folder (e.g. `C:\privatewhisper`).
2. Install the VC++ Redistributable (link above) if you don't have it.
3. Run `run.bat` (shows progress in a console) or `privatewhisper.exe`.
4. **First run** downloads the model and GPU runtime into `%LOCALAPPDATA%\privatewhisper\` — give it a few minutes. Later runs are instant.
5. A tray icon appears. Press **`Ctrl+Space`**, speak, press **`Ctrl+Space`** again → the text is typed into the focused window.

> Two release flavors: a **lite** zip (just the exe; downloads the runtime on first run) and a **full** zip (runtime DLLs bundled; only the model is downloaded).

## How it works

```
privatewhisper.exe  (tao event loop + tray + global hotkey)
├─ config   — config.toml (hotkey, provider, …)
├─ model    — first-run download of the Parakeet v3 fp16 model
├─ runtime  — first-run download of the ONNX Runtime GPU stack
├─ audio    — cpal capture → mono → resample to 16 kHz
├─ asr      — parakeet-rs (ONNX Runtime, load-dynamic) → text
├─ inject   — clipboard + Ctrl+V (layout-independent), restore clipboard
├─ overlay  — softbuffer waveform window (non-activating, click-through)
└─ app      — Idle → Recording → Transcribing state machine
```

**Blackwell (RTX 50-series) note.** Stock ONNX Runtime GPU builds don't ship `sm_120` kernels, so the CUDA provider fails with `cudaErrorNoKernelImageForDevice` on RTX 50-series cards. privatewhisper solves this by building [`parakeet-rs`](https://github.com/altunenes/parakeet-rs) with `load-dynamic` and pointing `ORT_DYLIB_PATH` at Microsoft's official **CUDA-13** ONNX Runtime build (the `onnxruntime-cuda-13` Azure feed) plus the matching NVIDIA CUDA 13 / cuDNN 9 DLLs — all fetched on first run.

## Build from source

```bash
# Linux/macOS dev: core logic + CPU ASR are testable without a GPU
cargo test
cargo run --example spike   # load a model + transcribe a wav

# Produce the Windows .exe (cross-compile from Linux via mingw-w64)
rustup target add x86_64-pc-windows-gnu
sudo apt install -y gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/privatewhisper.exe  (~25 MB, self-contained)
```

Or build natively on Windows with the MSVC toolchain: `cargo build --release`.

## Configuration

`%APPDATA%\privatewhisper\config.toml` (created on first run):

| key | default | meaning |
|---|---|---|
| `hotkey` | `"Ctrl+Space"` | global toggle hotkey |
| `execution_provider` | `"cuda"` | `"cuda"` (GPU) or `"cpu"` |
| `paste_delay_ms` | `80` | delay before restoring the clipboard |
| `ort_dylib_path` | unset | override the ONNX Runtime DLL (skips auto-download) |

## Acknowledgements

- [NVIDIA Parakeet TDT 0.6b v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) and the ONNX conversions ([istupakov](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx), [grikdotnet](https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16)).
- [`parakeet-rs`](https://github.com/altunenes/parakeet-rs) and [ONNX Runtime](https://onnxruntime.ai/).

## License

MIT — see [LICENSE](LICENSE). Model weights and runtime libraries are downloaded at runtime and carry their own licenses (NVIDIA / Microsoft).
