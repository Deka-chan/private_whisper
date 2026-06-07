use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    run()
}

#[cfg(target_os = "windows")]
fn run() -> anyhow::Result<()> {
    windows_app::run()
}

#[cfg(not(target_os = "windows"))]
fn run() -> anyhow::Result<()> {
    eprintln!("privatewhisper: full tray dictation app is Windows-only");
    Ok(())
}

#[cfg_attr(not(any(test, target_os = "windows")), allow(dead_code))]
fn default_model_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| {
            dirs.data_local_dir()
                .join("privatewhisper")
                .join("models")
                .join("parakeet-v3")
        })
        .unwrap_or_else(|| PathBuf::from("./models/parakeet-v3"))
}

#[cfg(target_os = "windows")]
mod windows_app {
    use std::{
        path::{Path, PathBuf},
        process::Command,
        sync::{
            atomic::{AtomicU32, Ordering},
            mpsc, Arc,
        },
        thread,
        time::{Duration, Instant},
    };

    use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
    use privatewhisper::{
        app::{self, Action, Event as AppEvent, State},
        asr::Asr,
        audio::Recorder,
        config::Config,
        inject, model,
        overlay::Overlay,
        tray,
    };
    use tao::{
        event::{Event, StartCause},
        event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    };
    use tray_icon::menu::MenuEvent;

    use crate::default_model_dir;

    enum UiEvent {
        Hotkey(global_hotkey::GlobalHotKeyEvent),
        Menu(tray_icon::menu::MenuEvent),
        Transcribed(String),
        Failed(String),
    }

    struct TranscriptionJob {
        samples: Vec<f32>,
        sample_rate: u32,
    }

    pub fn run() -> anyhow::Result<()> {
        let config = Config::load_or_create()?;
        ensure_runtime_env(&config)?;

        let model_dir = config.model_dir.clone().unwrap_or_else(default_model_dir);
        ensure_model(&model_dir)?;

        let asr = Asr::load(&model_dir, config.execution_provider)?;
        log::info!("ASR ready on {:?}", asr.provider);

        run_event_loop(config, model_dir, asr)
    }

    fn ensure_runtime_env(config: &Config) -> anyhow::Result<()> {
        // 1) Explicit override in config wins.
        if let Some(path) = &config.ort_dylib_path {
            std::env::set_var("ORT_DYLIB_PATH", path);
            if let Some(dir) = &config.ort_lib_dir {
                prepend_path_env(dir);
            } else if let Some(dir) = path.parent() {
                prepend_path_env(dir);
            }
            return Ok(());
        }

        // 2) Already provided via the environment (e.g. a bundled run.bat that sets
        //    ORT_DYLIB_PATH next to the exe). Respect it; just make sure its dir is on PATH.
        if let Ok(existing) = std::env::var("ORT_DYLIB_PATH") {
            if let Some(dir) = std::path::Path::new(&existing).parent() {
                prepend_path_env(dir);
            }
            return Ok(());
        }

        // 3) Auto-provision: download the GPU runtime to %LOCALAPPDATA%/privatewhisper/runtime.
        let runtime_dir = directories::BaseDirs::new()
            .map(|d| d.data_local_dir().join("privatewhisper").join("runtime"))
            .unwrap_or_else(|| std::path::PathBuf::from("./runtime"));
        log::info!(
            "provisioning GPU runtime into {} (first run downloads ~1.2 GB) ...",
            runtime_dir.display()
        );
        let dylib = privatewhisper::runtime::ensure(&runtime_dir, &mut |i, n, done, total| {
            log::info!("runtime wheel {}/{}: {}/{} bytes", i + 1, n, done, total);
        })?;
        std::env::set_var("ORT_DYLIB_PATH", &dylib);
        prepend_path_env(&runtime_dir);
        log::info!("ORT_DYLIB_PATH={}", dylib.display());
        Ok(())
    }

    fn prepend_path_env(dir: &Path) {
        let mut entries = vec![dir.to_path_buf()];
        if let Some(existing) = std::env::var_os("PATH") {
            entries.extend(std::env::split_paths(&existing));
        }
        match std::env::join_paths(entries) {
            Ok(path) => std::env::set_var("PATH", path),
            Err(err) => log::warn!(
                "could not prepend ONNX Runtime DLL directory {} to PATH: {err}",
                dir.display()
            ),
        }
    }

    fn ensure_model(model_dir: &Path) -> anyhow::Result<()> {
        model::ensure(model_dir, &mut |i, n, done, total| {
            log::info!("model file {}/{}: {}/{} bytes", i + 1, n, done, total);
        })
    }

    fn run_event_loop(config: Config, model_dir: PathBuf, asr: Asr) -> anyhow::Result<()> {
        let mut event_loop = EventLoopBuilder::<UiEvent>::with_user_event();
        let event_loop = event_loop.build();
        let proxy = event_loop.create_proxy();

        GlobalHotKeyEvent::set_event_handler(Some({
            let proxy = proxy.clone();
            move |event| {
                let _ = proxy.send_event(UiEvent::Hotkey(event));
            }
        }));

        MenuEvent::set_event_handler(Some({
            let proxy = proxy.clone();
            move |event| {
                let _ = proxy.send_event(UiEvent::Menu(event));
            }
        }));

        let hotkey: HotKey = config.hotkey.parse()?;
        let hotkeys = GlobalHotKeyManager::new()?;
        hotkeys.register(hotkey)?;
        log::info!("registered global hotkey {}", hotkey);

        let tray = tray::build()?;
        let level = Arc::new(AtomicU32::new(0));
        let mut overlay = Overlay::new(&event_loop, level.clone())?;
        let (job_tx, job_rx) = mpsc::channel();
        spawn_transcription_worker(asr, config.paste_delay_ms, proxy.clone(), job_rx);

        let config_path = Config::default_path()?;
        let input_device = config.input_device.clone();
        let mut state = State::Idle;
        let mut recorder: Option<Recorder> = None;

        event_loop.run(move |event, _, control_flow| {
            let mut exit = false;

            match event {
                Event::NewEvents(StartCause::Init) => {
                    update_tray(&tray, state);
                }
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                    overlay.tick();
                }
                Event::UserEvent(UiEvent::Hotkey(event)) => {
                    if event.id == hotkey.id() && matches!(event.state(), HotKeyState::Pressed) {
                        handle_hotkey(
                            &mut state,
                            &mut recorder,
                            input_device.as_deref(),
                            &level,
                            &job_tx,
                            &tray,
                        );
                        overlay.set_state(state);
                    }
                }
                Event::UserEvent(UiEvent::Transcribed(text)) => {
                    log::info!("transcription completed: {} bytes", text.len());
                    let (next, action) = app::transition(state, AppEvent::TranscriptionDone);
                    state = next;
                    log_unexpected_action(action);
                    update_tray(&tray, state);
                    overlay.set_state(state);
                }
                Event::UserEvent(UiEvent::Failed(error)) => {
                    log::error!("transcription failed: {error}");
                    let (next, action) = app::transition(state, AppEvent::TranscriptionFailed);
                    state = next;
                    log_unexpected_action(action);
                    update_tray(&tray, state);
                    overlay.set_state(state);
                }
                Event::UserEvent(UiEvent::Menu(event)) => {
                    if event.id == tray.quit_id {
                        exit = true;
                    } else if event.id == tray.open_config_id {
                        if let Err(err) = open_path(&config_path) {
                            log::error!("failed to open config {}: {err}", config_path.display());
                        }
                    } else if event.id == tray.redownload_id {
                        let model_dir = model_dir.clone();
                        let proxy = proxy.clone();
                        thread::spawn(move || {
                            if let Err(err) = ensure_model(&model_dir) {
                                let _ = proxy.send_event(UiEvent::Failed(err.to_string()));
                            }
                        });
                    } else if event.id == tray.provider_toggle_id {
                        log::info!(
                            "provider toggle clicked; runtime provider changes require restart"
                        );
                    }
                }
                Event::RedrawRequested(window_id) if window_id == overlay.id() => {
                    overlay.redraw();
                }
                _ => {}
            }

            let _keep_hotkey_manager_alive = &hotkeys;

            *control_flow = if exit {
                ControlFlow::Exit
            } else if overlay.is_active() {
                ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33))
            } else {
                ControlFlow::Wait
            };
        });
    }

    fn spawn_transcription_worker(
        mut asr: Asr,
        paste_delay_ms: u64,
        proxy: EventLoopProxy<UiEvent>,
        job_rx: mpsc::Receiver<TranscriptionJob>,
    ) {
        thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let result = asr
                    .transcribe(job.samples, job.sample_rate)
                    .and_then(|text| {
                        inject::paste(&text, paste_delay_ms)?;
                        Ok(text)
                    });

                let event = match result {
                    Ok(text) => UiEvent::Transcribed(text),
                    Err(err) => UiEvent::Failed(err.to_string()),
                };
                if proxy.send_event(event).is_err() {
                    break;
                }
            }
        });
    }

    fn handle_hotkey(
        state: &mut State,
        recorder: &mut Option<Recorder>,
        input_device: Option<&str>,
        level: &Arc<AtomicU32>,
        job_tx: &mpsc::Sender<TranscriptionJob>,
        tray: &tray::Tray,
    ) {
        let previous = *state;
        let (next, action) = app::transition(*state, AppEvent::HotkeyPressed);
        *state = next;
        update_tray(tray, *state);

        match action {
            Action::None => {}
            Action::StartRecording => match Recorder::start(input_device, level.clone()) {
                Ok(active) => {
                    *recorder = Some(active);
                }
                Err(err) => {
                    log::error!("failed to start recording: {err}");
                    level.store(0f32.to_bits(), Ordering::Relaxed);
                    *state = previous;
                    update_tray(tray, *state);
                }
            },
            Action::StopAndTranscribe => {
                level.store(0f32.to_bits(), Ordering::Relaxed);
                if let Some(active) = recorder.take() {
                    let (samples, sample_rate) = active.stop();
                    if let Err(err) = job_tx.send(TranscriptionJob {
                        samples,
                        sample_rate,
                    }) {
                        log::error!("failed to queue transcription job: {err}");
                        *state = State::Idle;
                        update_tray(tray, *state);
                    }
                } else {
                    log::warn!("hotkey requested transcription without an active recorder");
                    level.store(0f32.to_bits(), Ordering::Relaxed);
                    *state = previous;
                    update_tray(tray, *state);
                }
            }
        }
    }

    fn update_tray(tray: &tray::Tray, state: State) {
        if let Err(err) = tray::set_state(tray, state) {
            log::warn!("failed to update tray state to {:?}: {err}", state);
        }
    }

    fn log_unexpected_action(action: Action) {
        if !matches!(action, Action::None) {
            log::warn!("completion event produced unexpected action: {:?}", action);
        }
    }

    fn open_path(path: &Path) -> anyhow::Result<()> {
        Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_model_dir_uses_privatewhisper_parakeet_v3_leaf() {
        let path = super::default_model_dir();
        let rendered = path.to_string_lossy();

        assert!(rendered.contains("privatewhisper"));
        assert!(rendered.ends_with("models/parakeet-v3"));
    }
}
