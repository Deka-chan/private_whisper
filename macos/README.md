# privatewhisper — macOS

A **macOS adaptation** of [privatewhisper](https://github.com/Deka-chan/private_whisper), reimagined for Apple Silicon. Press a hotkey, speak, and your words are transcribed **entirely on-device** and typed at the cursor — nothing leaves your machine.

The original is a Rust/Windows app running NVIDIA Parakeet on CUDA. Apple Silicon has a different stack, so this is a from-scratch Swift app that keeps the same spirit (global hotkey, menu‑bar presence, live waveform, fully local) while using the Apple‑native path: **WhisperKit (CoreML / Apple Neural Engine)** with **Whisper large‑v3‑turbo**.

> Status: works well on macOS 26 / Apple M2. Should run on any Apple Silicon Mac, macOS 14+.

## Features

- 🔒 **Fully local** — audio and text never leave your Mac.
- 🎙️ **Two gestures on one key** — **hold** the hotkey for push‑to‑talk, or **double‑tap** to toggle dictation until you press again.
- ⚡ **Fast & accurate** — Whisper **large‑v3‑turbo** via WhisperKit on the **Neural Engine**; sub‑second transcription on an M2 Air.
- 🌍 **Multilingual** — Whisper's ~99 languages with auto‑detection (UI exposes Auto / Russian / English).
- 〰️ **Live equalizer overlay** — a click‑through, non‑activating panel that reacts to your voice, then shows a "processing" sweep while transcribing.
- ⌨️ **Clean insertion** — text is typed via synthesized Unicode key events, so your **clipboard is never touched**.
- 🧹 **No tail hallucinations** — trailing silence is trimmed and stray endings like "Thank you" are stripped.
- 🗂️ **History** — your last 5 dictations, copyable and editable from the menu bar.
- ⚙️ **Settings** — language and hotkey picker (fn / Right ⌘ / Right ⌥ / Right ⌃); optional launch‑at‑login.

## Requirements

- Apple Silicon Mac, macOS 14+
- Xcode (provides the Swift toolchain). If `xcode-select` points at the Command Line Tools, prefix Swift commands with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`.

## Build & run

```bash
cd macos
./scripts/make_app.sh release
open build/PrivateWhisper.app
```

`make_app.sh` builds the SwiftPM executable, assembles a `.app` bundle (with `Info.plist`) and code‑signs it. It auto‑selects a stable signing identity (Apple Development / Developer ID) if one exists, otherwise falls back to ad‑hoc.

> Ad‑hoc signing changes the app's identity on every rebuild, which makes macOS forget previously‑granted permissions. A stable signing identity avoids that — recommended if you iterate on the code.

## First run

1. Grant **Microphone** and **Accessibility** (and **Input Monitoring**) when prompted, then relaunch.
2. WhisperKit downloads and compiles the **large‑v3‑turbo** CoreML model on first launch (progress shows next to the menu‑bar icon). One time.
3. If the **fn (🌐)** key doesn't trigger: System Settings → Keyboard → "Press 🌐 to" → **Do Nothing**.

## Usage

- **Hold** the hotkey → push‑to‑talk (records while held).
- **Double‑tap** → record until you tap again.

## Tests

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift test --filter PrivateWhisperCoreTests
```

See [DESIGN.md](DESIGN.md) for the architecture.

## License

MIT (same as the upstream project). Model weights and the WhisperKit/CoreML runtime carry their own licenses.
