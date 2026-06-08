# macOS Port Integration + v0.2.0 Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Accept PR #1 (self-contained macOS/Swift port under `macos/`), set up a reproducible CI release that builds both platforms, and publish a single combined release `v0.2.0` with the Windows `.exe` and macOS `.app`.

**Architecture:** Monorepo. PR #1 is squash-merged into `main` (brings `macos/`). A `release.yml` GitHub Actions workflow builds the macOS `.app` on a `macos-14` runner (ad-hoc signed) and the Windows lite `.exe` on `windows-latest`; the same workflow runs build+test on every PR (no publish) and publishes a GitHub Release only on a `v*` tag. Version is bumped to 0.2.0 in both apps. README gains a Downloads section with macOS quarantine instructions.

**Tech Stack:** Rust (Cargo, `parakeet-rs` with `load-dynamic`), Swift (SwiftPM, WhisperKit/CoreML), GitHub Actions (`softprops/action-gh-release`, `actions/upload-artifact@v4`).

---

## Context for the implementer

- Dev machine is Linux/WSL: **the macOS Swift app cannot be built or tested locally.** macOS verification happens only in CI.
- Current versions are `0.1.0` in `Cargo.toml` and `macos/Resources/Info.plist` (the latter arrives via the PR #1 merge).
- `dist/` is gitignored, so the Windows lite `run.bat`/`README.txt` do not exist as tracked files yet — Task 4 adds them under `packaging/win/`.
- CI produces only the **lite** Windows zip (just the exe; downloads runtime on first run). The "full" DLL-bundled zip is out of scope for CI (those DLLs are gitignored/downloaded at runtime).
- Current working branch is `macos-port-integration` (off pre-merge `main`), containing only the design-doc commit.

## File Structure

- Create: `.github/workflows/release.yml` — the build+release workflow.
- Create: `packaging/win/run.bat` — launcher for the Windows lite distribution.
- Create: `packaging/win/README.txt` — end-user readme inside the Windows lite zip.
- Modify: `Cargo.toml` — version `0.1.0` → `0.2.0`.
- Modify: `macos/Resources/Info.plist` — `CFBundleVersion` / `CFBundleShortVersionString` `0.1.0` → `0.2.0` (file exists after Task 2).
- Modify: `README.md` — add Downloads section + macOS quarantine note.

---

## Task 1: Light review of PR #1

**Files:** none modified (read-only review).

- [ ] **Step 1: Fetch and check out the PR locally**

Run:
```bash
gh pr checkout 1
```
Expected: switches to branch `macos-port` with the PR's 41 files present under `macos/`.

- [ ] **Step 2: Confirm the PR does not touch the Rust project**

Run:
```bash
git diff --name-only main...HEAD | grep -v '^macos/' | grep -v '^README.md$'
```
Expected: **no output** (only `macos/**` and `README.md` changed). If anything else prints, stop and report it.

- [ ] **Step 3: Review the security-sensitive Swift files**

Read these files and confirm there is no telemetry, no unexpected network egress, and behavior matches the PR description:
```bash
sed -n '1,200p' macos/Sources/PrivateWhisperApp/TextInjector.swift
sed -n '1,200p' macos/Sources/PrivateWhisperApp/Permissions.swift
sed -n '1,200p' macos/Sources/PrivateWhisperApp/WhisperKitEngine.swift
```
Expected/verify: `TextInjector` synthesizes key events and never reads/writes `NSPasteboard`; `Permissions` requests mic/accessibility only; `WhisperKitEngine` downloads the model from argmax/HuggingFace (WhisperKit defaults) and makes no other network calls. Note any concern; if serious, stop and report before merging.

- [ ] **Step 4: Confirm the tests are real (not stubs)**

Run:
```bash
ls macos/Tests/PrivateWhisperCoreTests/
grep -rc 'func test' macos/Tests/PrivateWhisperCoreTests/ | sort
```
Expected: 9 test files, each with one or more `func test...` (≈39 total). Skim 2-3 files to confirm assertions are meaningful.

- [ ] **Step 5: Return to the integration branch**

Run:
```bash
git checkout macos-port-integration
```
Expected: back on `macos-port-integration` (no commit made in this task).

---

## Task 2: Squash-merge PR #1 into main

**Files:** none locally (merge happens on GitHub).

- [ ] **Step 1: Squash-merge the PR**

Run:
```bash
gh pr merge 1 --squash --subject "feat(macos): add Apple Silicon port (WhisperKit/CoreML)" --body "Native Swift menu-bar app under macos/. From PR #1 by @Moooonwalker5."
```
Expected: `✓ Squashed and merged pull request #1`.

- [ ] **Step 2: Sync local main**

Run:
```bash
git checkout main && git pull --ff-only origin main
```
Expected: `main` now contains the `macos/` directory.

- [ ] **Step 3: Verify macos/ landed on main**

Run:
```bash
ls macos/Resources/Info.plist && ls macos/Package.swift
```
Expected: both paths exist.

- [ ] **Step 4: Rebase the integration branch onto updated main**

Run:
```bash
git checkout macos-port-integration && git rebase main
```
Expected: clean rebase; the branch now has `macos/` plus the design-doc commit. (No conflicts expected — the design doc is the only change.)

---

## Task 3: Bump versions to 0.2.0

**Files:**
- Modify: `Cargo.toml`
- Modify: `macos/Resources/Info.plist`

- [ ] **Step 1: Bump the Rust package version**

In `Cargo.toml`, change:
```toml
version = "0.1.0"
```
to:
```toml
version = "0.2.0"
```

- [ ] **Step 2: Refresh Cargo.lock**

Run:
```bash
cargo update -p privatewhisper --precise 0.2.0 2>/dev/null; cargo metadata --format-version 1 >/dev/null
```
Expected: `Cargo.lock` shows `name = "privatewhisper"` with `version = "0.2.0"`. Verify:
```bash
grep -A1 'name = "privatewhisper"' Cargo.lock | grep '0.2.0'
```
Expected: prints the `version = "0.2.0"` line.

- [ ] **Step 3: Bump the macOS bundle version**

In `macos/Resources/Info.plist`, change both occurrences of `0.1.0` to `0.2.0`:
```xml
    <key>CFBundleVersion</key><string>0.2.0</string>
    <key>CFBundleShortVersionString</key><string>0.2.0</string>
```

- [ ] **Step 4: Verify the bumps**

Run:
```bash
grep '^version' Cargo.toml && grep -o '0\.2\.0' macos/Resources/Info.plist | wc -l
```
Expected: `version = "0.2.0"` and the count `2`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock macos/Resources/Info.plist
git commit -m "chore: bump version to 0.2.0 (Windows + macOS)"
```

---

## Task 4: Add Windows lite packaging files

**Files:**
- Create: `packaging/win/run.bat`
- Create: `packaging/win/README.txt`

- [ ] **Step 1: Create `packaging/win/run.bat`**

```bat
@echo off
rem privatewhisper (lite) — does NOT set ORT_DYLIB_PATH, so on first run the app
rem downloads the GPU runtime itself into %LOCALAPPDATA%\privatewhisper\runtime.
setlocal
set "RUST_LOG=info"
"%~dp0privatewhisper.exe"
echo.
echo (privatewhisper exited — window kept open so you can read any messages)
pause
```

- [ ] **Step 2: Create `packaging/win/README.txt`**

```text
privatewhisper (lite) — локальная диктовка на GPU, лёгкий дистрибутив
=====================================================================

ОТЛИЧИЕ ОТ ПОЛНОГО БАНДЛА
  Здесь только exe (~26 МБ). Тяжёлое приложение скачивает само ПРИ ПЕРВОМ
  ЗАПУСКЕ в %LOCALAPPDATA%\privatewhisper\:
    • модель Parakeet TDT v3 (fp16) — ~1.24 ГБ  -> ...\models\parakeet-v3
    • GPU-рантайм (onnxruntime CUDA-13 + CUDA13/cuDNN9) — ~1.2 ГБ -> ...\runtime
  Итого первый запуск тянет ~2.5 ГБ. Дальше запуски быстрые, без загрузок.

ТРЕБОВАНИЯ
  • Windows 10/11 x64
  • NVIDIA RTX 50-й серии (Blackwell) + свежий драйвер (CUDA 13-capable)
  • Microsoft Visual C++ Redistributable 2015–2022 (x64):
        https://aka.ms/vs/17/release/vc_redist.x64.exe
  • ~2.5 ГБ трафика на первый запуск

ЗАПУСК
  1. Скопируй папку на диск Windows (например C:\privatewhisper).
  2. Запусти run.bat (покажет прогресс загрузки в консоли).
  3. Дождись, пока докачаются рантайм и модель (видно в логах).
  4. Появится иконка в трее. Ctrl+Space — старт записи (внизу побежит
     waveform-индикатор), Ctrl+Space ещё раз — распознавание и вставка по курсору.

ЕСЛИ НУЖНО БЕЗ ЗАГРУЗКИ
  Используй полный бандл (папка privatewhisper рядом), где GPU-рантайм уже лежит
  рядом с exe и его run.bat указывает на него — тогда качается только модель.

НАСТРОЙКИ: %APPDATA%\privatewhisper\config.toml (hotkey, execution_provider, ...).
Логи: запусти из консоли, смотри строки "runtime wheel i/N", "model file ...",
"ASR ready on Cuda".
```

- [ ] **Step 3: Verify files exist**

Run:
```bash
ls packaging/win/run.bat packaging/win/README.txt
```
Expected: both paths print.

- [ ] **Step 4: Commit**

```bash
git add packaging/win/run.bat packaging/win/README.txt
git commit -m "build(win): track lite-distribution run.bat + README for CI packaging"
```

---

## Task 5: Add the CI release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: release

on:
  push:
    tags:
      - "v*"
  pull_request:
  workflow_dispatch:

permissions:
  contents: write

jobs:
  macos:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - name: Swift version
        run: swift --version
      - name: Core tests
        working-directory: macos
        run: swift test --filter PrivateWhisperCoreTests
      - name: Build .app (ad-hoc signed)
        working-directory: macos
        run: ./scripts/make_app.sh release
      - name: Zip .app
        working-directory: macos/build
        run: ditto -c -k --keepParent PrivateWhisper.app "$GITHUB_WORKSPACE/PrivateWhisper-macOS-arm64.zip"
      - uses: actions/upload-artifact@v4
        with:
          name: PrivateWhisper-macOS-arm64
          path: PrivateWhisper-macOS-arm64.zip
          if-no-files-found: error

  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build (release, lite exe)
        run: cargo build --release
      - name: Assemble lite package
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force -Path staging\privatewhisper-lite | Out-Null
          Copy-Item target\release\privatewhisper.exe staging\privatewhisper-lite\
          Copy-Item packaging\win\run.bat             staging\privatewhisper-lite\
          Copy-Item packaging\win\README.txt          staging\privatewhisper-lite\
          Compress-Archive -Path staging\privatewhisper-lite -DestinationPath PrivateWhisper-lite-win-x64.zip -Force
      - uses: actions/upload-artifact@v4
        with:
          name: PrivateWhisper-lite-win-x64
          path: PrivateWhisper-lite-win-x64.zip
          if-no-files-found: error

  release:
    needs: [macos, windows]
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true
      - name: Publish GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: artifacts/*.zip
          generate_release_notes: true
          fail_on_unmatched_files: true
```

- [ ] **Step 2: Lint the YAML locally**

Run:
```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML OK')"
```
Expected: `YAML OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: build macOS .app + Windows lite exe; publish release on v* tag"
```

---

## Task 6: Update README with Downloads + macOS instructions

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add a Downloads section after the intro**

Insert the following block immediately after line 7 (`---`) and before `## Features`:

```markdown
## Download

Grab the latest build from the [**Releases**](../../releases/latest) page:

- **Windows** — `PrivateWhisper-lite-win-x64.zip` (the ~26 MB lite exe; downloads the model + GPU runtime on first run). See [Install & run](#install--run).
- **macOS (Apple Silicon)** — `PrivateWhisper-macOS-arm64.zip`. The `.app` is **ad-hoc signed**, so Gatekeeper will block it on first launch. Clear the quarantine flag once after unzipping:

  ```bash
  xattr -dr com.apple.quarantine PrivateWhisper.app
  ```

  …or right-click the app → **Open** → **Open**. Then grant **Microphone** and **Accessibility** permissions when prompted. See [`macos/README.md`](macos/README.md) for details.

---
```

- [ ] **Step 2: Verify the section renders and links resolve**

Run:
```bash
grep -n '## Download' README.md && grep -n 'com.apple.quarantine' README.md
```
Expected: both lines print.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add Downloads section (Windows + macOS) with quarantine note"
```

---

## Task 7: Open the release-prep PR and verify CI builds

**Files:** none.

- [ ] **Step 1: Push the branch**

Run:
```bash
git push -u origin macos-port-integration
```
Expected: branch pushed; `release.yml` triggers on the resulting PR.

- [ ] **Step 2: Open the PR**

Run:
```bash
gh pr create --base main --head macos-port-integration \
  --title "Release prep: v0.2.0 (CI build for Windows + macOS)" \
  --body "Adds CI release workflow, Windows lite packaging, version bump to 0.2.0, and Downloads docs. Builds both platforms on this PR to verify before tagging."
```
Expected: PR URL printed.

- [ ] **Step 3: Watch CI and confirm both build jobs pass**

Run:
```bash
gh pr checks --watch
```
Expected: `macos` and `windows` jobs both **pass** (the `release` job is skipped — it only runs on a tag). If a build fails, debug it (see Risks) before continuing. This is the gate that proves the macOS and Windows builds work without cutting a real release.

- [ ] **Step 4: (Optional) Download and sanity-check the CI artifacts**

Run:
```bash
gh run download "$(gh run list --branch macos-port-integration --limit 1 --json databaseId --jq '.[0].databaseId')" --dir /tmp/ci-artifacts
ls -la /tmp/ci-artifacts/**/*.zip
```
Expected: both `PrivateWhisper-macOS-arm64.zip` and `PrivateWhisper-lite-win-x64.zip` are present and non-empty.

---

## Task 8: Merge to main and cut release v0.2.0

**Files:** none.

- [ ] **Step 1: Merge the release-prep PR**

Run:
```bash
gh pr merge macos-port-integration --squash --delete-branch
```
Expected: `✓ Squashed and merged`. 

- [ ] **Step 2: Sync local main**

Run:
```bash
git checkout main && git pull --ff-only origin main
```
Expected: `main` has the workflow, packaging files, version bump, and README changes.

- [ ] **Step 3: Tag and push v0.2.0**

Run:
```bash
git tag -a v0.2.0 -m "privatewhisper v0.2.0 — Windows + macOS (Apple Silicon)"
git push origin v0.2.0
```
Expected: tag pushed; the `release` workflow runs all three jobs.

- [ ] **Step 4: Watch the release run**

Run:
```bash
gh run watch "$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
```
Expected: `macos`, `windows`, and `release` jobs all pass.

- [ ] **Step 5: Verify the published release**

Run:
```bash
gh release view v0.2.0 --json tagName,assets --jq '{tag: .tagName, assets: [.assets[].name]}'
```
Expected: `tag` is `v0.2.0` and `assets` lists both `PrivateWhisper-macOS-arm64.zip` and `PrivateWhisper-lite-win-x64.zip`.

- [ ] **Step 6: Final confirmation**

Open the release page and confirm release notes are populated (auto-generated) and both artifacts download:
```bash
gh release view v0.2.0 --web
```

---

## Risks & fallbacks

- **macOS build fails in CI (Xcode/Swift/WhisperKit resolution):** check `swift --version` step output; if `make_app.sh`'s `DEVELOPER_DIR` default doesn't match the runner, add `env: DEVELOPER_DIR: /Applications/Xcode.app/Contents/Developer` (or a `sudo xcode-select -s` step) to the `macos` job.
- **Windows `cargo build --release` fails on `windows-latest`:** the `load-dynamic` feature means no CUDA SDK is needed at build time; if a native-MSVC issue appears, fall back to the documented mingw cross-compile (`x86_64-pc-windows-gnu`) on an `ubuntu-latest` job, or attach a locally-built Windows zip manually to the release.
- **Release job can't create the release:** ensure `permissions: contents: write` is present (it is) and that Actions are allowed to write to the repo (Settings → Actions → Workflow permissions).

## Self-review notes

- Spec coverage: Phase 1 → Task 1; Phase 2 → Task 2; Phase 3 → Task 3; Phase 4 (CI) → Task 5 (+ PR-time build verification in Task 7); Phase 5 (README) → Task 6; Phase 6 (cut release) → Task 8. Windows packaging (implied by the Windows CI job) → Task 4. All covered.
- Artifact names are consistent across Tasks 5, 7, 8: `PrivateWhisper-macOS-arm64.zip` and `PrivateWhisper-lite-win-x64.zip`.
- Version string `0.2.0` consistent across `Cargo.toml`, `Info.plist`, and the `v0.2.0` tag.
- No placeholders: all file contents and commands are concrete.
