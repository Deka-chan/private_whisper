# Design: Integrate macOS port (PR #1) + release v0.2.0

Date: 2026-06-08
Status: Approved (brainstorming)

## Goal

Accept PR #1 — a self-contained macOS / Apple Silicon port of privatewhisper
(native Swift app under `macos/`, does not touch the Rust/Windows code except one
line in the root `README.md`) — set up a **reproducible CI release**, and ship a
single combined release **`v0.2.0`** carrying two artifacts: the Windows `.exe`
and the macOS `.app`. The repository stays a **monorepo**.

## Hard constraint

The development machine is **Linux/WSL**. The Swift/macOS app **cannot be built
or tested locally** — it requires an Apple Silicon Mac with Xcode. Therefore the
macOS artifact must be produced by CI on a macOS runner, and we cannot personally
verify the macOS runtime behavior beyond reading the code.

## Decisions (resolved during brainstorming)

- **Review depth:** Light review (structure, README/DESIGN, tests, top-level
  security). No line-by-line audit.
- **Release scope:** Build macOS via CI (GitHub Actions macOS runner). Combined
  release with both platform artifacts.
- **Versioning:** Single shared tag `v0.2.0`; one GitHub Release with both
  artifacts.
- **macOS signing:** Ad-hoc (no Apple Developer secrets) + README instructions to
  remove the quarantine attribute.
- **Windows in CI:** Yes — both platforms build in CI for a single
  release-on-tag workflow.
- **Merge style:** Squash (linear history, consistent with existing commit style).
- **`CFBundleIdentifier`:** Leave as `com.abdulla.privatewhisper` (contributor
  attribution; not critical).

## Phases

### Phase 1 — Light review of PR #1 (before merge)
No line-by-line audit. Check:
- **Structure:** `PrivateWhisperCore` (pure logic + 39 tests) vs
  `PrivateWhisperApp` (AppKit shell) — boundaries as claimed.
- **Top-level security:** `TextInjector` (synthesizes key events, never touches
  the clipboard), `Permissions` (mic/accessibility), `WhisperKitEngine` (where it
  downloads the model — argmax/HuggingFace), and check for suspicious network
  calls / telemetry.
- **Self-containment:** confirm it really does not touch the root Rust project
  (other than one line in `README.md`).
- **Tests:** skim that they are meaningful (not empty stubs).

If something serious surfaces, stop and report before merging.

### Phase 2 — Merge PR into `main`
`gh pr merge 1 --squash`. PR is `MERGEABLE`, no conflicts.

### Phase 3 — Version bump to 0.2.0
- `Cargo.toml`: `0.1.0` → `0.2.0`
- `macos/Resources/Info.plist`: `CFBundleVersion` / `CFBundleShortVersionString`
  `0.1.0` → `0.2.0`
- `Cargo.lock` updates automatically.

### Phase 4 — CI (`.github/workflows/release.yml`, trigger on tag `v*`)
Two build jobs, both attaching artifacts to one GitHub Release:
- **macOS job** (`macos-14`, Apple Silicon):
  `swift test --filter PrivateWhisperCoreTests` →
  `./scripts/make_app.sh release` (ad-hoc sign) → zip the `.app` →
  `PrivateWhisper-macOS-arm64.zip`.
- **Windows job** (`windows-latest`):
  `cargo build --release` (the lite exe; `parakeet-rs` uses `load-dynamic`, so no
  CUDA SDK is needed at build time) → assemble lite folder
  (exe + `run.bat` + `README.txt`) → zip `PrivateWhisper-lite-win-x64.zip`.
- **release job:** `softprops/action-gh-release` creates release `v0.2.0`,
  attaches both zips.

Optional: a light `ci.yml` (on push/PR) that runs `swift test` on macOS so the
tests don't rot.

### Phase 5 — README / docs
- Root `README.md`: a Downloads section linking both release artifacts.
- macOS launch instructions (ad-hoc signing):
  `xattr -dr com.apple.quarantine PrivateWhisper.app` or right-click → Open.

### Phase 6 — Cut the release
Push tag `v0.2.0` → CI builds both → release is published. Verify both artifacts
are present and release notes are filled in.

## Out of scope
- Porting the CUDA/ONNX stack to macOS (the port uses WhisperKit/CoreML instead).
- Apple Developer ID signing / notarization (no paid account; ad-hoc + quarantine
  instructions instead).
- Any refactor of the existing Rust/Windows code.

## Risks
- We cannot verify macOS runtime behavior locally; we rely on the contributor's
  testing ("macOS 26 / M2") and CI build/test success.
- Ad-hoc signed `.app` triggers Gatekeeper warnings; mitigated by README
  instructions.
- Windows CI build of the Rust app is unproven on `windows-latest`; if it fails,
  fall back to attaching a locally-built Windows zip manually.
