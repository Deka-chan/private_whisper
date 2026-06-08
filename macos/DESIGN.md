# Design — privatewhisper for macOS

A local voice‑dictation menu‑bar app for Apple Silicon. This document describes the actual architecture as implemented.

## Why a separate implementation

The upstream app is Rust + NVIDIA Parakeet on ONNX Runtime/CUDA, targeting Windows. None of the GPU path applies on a Mac. Rather than port the CUDA stack, this version uses the Apple‑native route for best speed and battery on Apple Silicon:

- **Engine:** [WhisperKit](https://github.com/argmaxinc/argmax-oss-swift) — CoreML inference scheduled across the **Apple Neural Engine** / GPU.
- **Model:** Whisper **large‑v3‑turbo** (`large-v3-v20240930_turbo`) — 809M params with a distilled 4‑layer decoder: near‑large accuracy at a fraction of the cost. WhisperKit downloads and compiles it on first run.

## Module layout

The code is split into a pure, unit‑tested core and a thin AppKit shell.

### `PrivateWhisperCore` (pure logic, no UI — 39 unit tests)
- **StateMachine** — single owner of state transitions: `idle → recording → transcribing → inserting → idle`, with guards.
- **HotkeyGestureRecognizer** — turns one key's down/up events (+ host‑driven hold/double‑tap timers) into gestures: hold ⇒ push‑to‑talk, double‑tap ⇒ toggle. Timer‑free and fully testable.
- **AudioMath** — RMS level (for the overlay), interleaved→mono downmix, and trailing/leading **silence trimming** (kills Whisper's tail hallucinations).
- **TranscriptCleanup** — strips trailing hallucinated phrases ("Thank you", "Спасибо за просмотр", …) without touching legitimate text.
- **LanguageMode / HotkeyKey / Config / HistoryStore** — settings + last‑5 dictation history (persisted in `UserDefaults`).
- (Also `ModelCatalog` / `Checksum`, retained from the earlier ggml‑based approach.)

### `PrivateWhisperApp` (AppKit executable)
- **AppDelegate** — wires everything; `.accessory` activation policy (menu‑bar only).
- **MenuBarController** — `NSStatusItem`, state icon, menu (History / Settings / Launch‑at‑login / Quit).
- **HotkeyManager** — global `CGEvent` tap on `.flagsChanged`; maps the configured key (fn / right ⌘/⌥/⌃) to edge events feeding the recognizer; owns the timers.
- **AudioCapture** — `AVAudioEngine` → `AVAudioConverter` → 16 kHz mono Float; streams RMS to the overlay and PCM to the engine.
- **WhisperKitEngine** — `TranscriptionEngine` over WhisperKit; auto language detection for "Auto", explicit `ru`/`en` otherwise.
- **TextInjector** — types the result via synthesized Unicode `CGEvent`s (no clipboard involvement).
- **OverlayWindow** — borderless, click‑through, non‑activating `NSPanel`; an equalizer that reacts to voice, plus a "processing" sweep during transcription.
- **LoginItem** — `SMAppService` launch‑at‑login.
- **SettingsView / HistoryView** — SwiftUI windows.

## Data flow

```
hotkey (hold / double‑tap)
  → StateMachine: recording → AudioCapture (16 kHz mono) ──→ overlay (live level)
  → stop → trim silence → WhisperKitEngine.transcribe (ANE) → cleanup
  → type text via Unicode CGEvents → history.add → idle
```

## Permissions

Microphone (capture) and Accessibility / Input Monitoring (global hotkey tap + synthesized typing). The app guides the user to grant them and deep‑links to the right System Settings pane.

## Notes / trade‑offs

- An earlier iteration used whisper.cpp via SwiftWhisper with ggml models. That bundled whisper.cpp was too old for large‑v3‑turbo (128‑mel) and large‑v2 was slow on a fanless M2 Air, so the engine was moved to WhisperKit, which supports turbo natively and runs on the Neural Engine.
- Insertion uses Unicode keystroke synthesis rather than clipboard paste to avoid clobbering the user's clipboard and to sidestep paste/restore races.
