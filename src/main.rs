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
        sync::mpsc,
        thread,
    };

    use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
    use privatewhisper::{
        app::{self, Action, Event as AppEvent, State},
        asr::Asr,
        audio::Recorder,
        config::Config,
        inject, model, tray,
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
        configure_onnx_runtime(&config);

        let model_dir = config.model_dir.clone().unwrap_or_else(default_model_dir);
        ensure_model(&model_dir)?;

        let asr = Asr::load(&model_dir, config.execution_provider)?;
        log::info!("ASR ready on {:?}", asr.provider);

        run_event_loop(config, model_dir, asr)
    }

    fn configure_onnx_runtime(config: &Config) {
        if let Some(path) = &config.ort_dylib_path {
            std::env::set_var("ORT_DYLIB_PATH", path);
            log::info!("ORT_DYLIB_PATH={}", path.display());
        }

        if let Some(dir) = &config.ort_lib_dir {
            prepend_path_env(dir);
            log::info!(
                "prepended ONNX Runtime DLL directory to PATH: {}",
                dir.display()
            );
        }
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
        let (job_tx, job_rx) = mpsc::channel();
        spawn_transcription_worker(asr, config.paste_delay_ms, proxy.clone(), job_rx);

        let config_path = Config::default_path()?;
        let input_device = config.input_device.clone();
        let mut state = State::Idle;
        let mut recorder: Option<Recorder> = None;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => {
                    update_tray(&tray, state);
                }
                Event::UserEvent(UiEvent::Hotkey(event)) => {
                    if event.id == hotkey.id() && matches!(event.state(), HotKeyState::Pressed) {
                        handle_hotkey(
                            &mut state,
                            &mut recorder,
                            input_device.as_deref(),
                            &job_tx,
                            &tray,
                        );
                    }
                }
                Event::UserEvent(UiEvent::Transcribed(text)) => {
                    log::info!("transcription completed: {} bytes", text.len());
                    let (next, action) = app::transition(state, AppEvent::TranscriptionDone);
                    state = next;
                    log_unexpected_action(action);
                    update_tray(&tray, state);
                }
                Event::UserEvent(UiEvent::Failed(error)) => {
                    log::error!("transcription failed: {error}");
                    let (next, action) = app::transition(state, AppEvent::TranscriptionFailed);
                    state = next;
                    log_unexpected_action(action);
                    update_tray(&tray, state);
                }
                Event::UserEvent(UiEvent::Menu(event)) => {
                    if event.id == tray.quit_id {
                        *control_flow = ControlFlow::Exit;
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
                _ => {}
            }

            let _keep_hotkey_manager_alive = &hotkeys;
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
        job_tx: &mpsc::Sender<TranscriptionJob>,
        tray: &tray::Tray,
    ) {
        let previous = *state;
        let (next, action) = app::transition(*state, AppEvent::HotkeyPressed);
        *state = next;
        update_tray(tray, *state);

        match action {
            Action::None => {}
            Action::StartRecording => match Recorder::start(input_device) {
                Ok(active) => {
                    *recorder = Some(active);
                }
                Err(err) => {
                    log::error!("failed to start recording: {err}");
                    *state = previous;
                    update_tray(tray, *state);
                }
            },
            Action::StopAndTranscribe => {
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
