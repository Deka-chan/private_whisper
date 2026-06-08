import AppKit
import PrivateWhisperCore

@MainActor final class AppDelegate: NSObject, NSApplicationDelegate {
    private let config = Config()
    private let stateMachine = StateMachine()
    private let menuBar = MenuBarController()
    private let overlay = OverlayWindow()
    private let audio = AudioCapture()
    private let hotkeys = HotkeyManager()
    private let settingsWindow = SettingsWindowController()
    private let history = HistoryStore()
    private let historyWindow = HistoryWindowController()
    private var engine: WhisperKitEngine!
    private var injector: TextInjector!

    func applicationDidFinishLaunching(_ notification: Notification) {
        injector = TextInjector()
        engine = WhisperKitEngine()

        stateMachine.onChange = { [weak self] in self?.menuBar.setIcon(for: $0) }
        menuBar.onQuit = { NSApp.terminate(nil) }
        menuBar.setLaunchAtLogin(LoginItem.isEnabled)
        menuBar.onToggleLaunchAtLogin = { [weak self] in
            let now = LoginItem.setEnabled(!LoginItem.isEnabled)
            self?.menuBar.setLaunchAtLogin(now)
        }
        menuBar.onOpenSettings = { [weak self] in
            guard let self else { return }
            self.settingsWindow.show(
                language: self.config.language,
                hotkey: self.config.hotkey,
                onLanguageChange: { [weak self] new in self?.config.language = new },
                onHotkeyChange: { [weak self] new in
                    self?.config.hotkey = new
                    self?.hotkeys.setKey(new)
                }
            )
        }
        menuBar.onOpenHistory = { [weak self] in
            guard let self else { return }
            self.historyWindow.show(
                items: self.history.items,
                onCopy: { text in
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(text, forType: .string)
                },
                onUpdate: { [weak self] index, text in self?.history.update(at: index, text: text) }
            )
        }

        audio.onLevel = { [weak self] level in
            DispatchQueue.main.async { self?.overlay.push(level: level) }
        }

        hotkeys.onGesture = { [weak self] gesture in
            DispatchQueue.main.async { self?.handle(gesture) }
        }

        Task { await bootstrap() }
    }

    private func bootstrap() async {
        if !Permissions.microphoneAuthorized() {
            _ = await Permissions.requestMicrophone()
        }
        if !Permissions.accessibilityTrusted(prompt: true) {
            alert("Нужен доступ Accessibility",
                  "Включи PrivateWhisper в Системные настройки → Конфиденциальность → Универсальный доступ, затем перезапусти.",
                  openPane: .accessibility)
        }
        // WhisperKit при первой загрузке сам скачает и скомпилирует модель
        // (large-v3-turbo) — это может занять время. Показываем статус.
        menuBar.setStatusText("⏳ модель…")
        do { try await engine.loadModel() }
        catch { alert("Не удалось загрузить модель", "\(error)", openPane: nil) }
        menuBar.setStatusText(nil)
        hotkeys.setKey(config.hotkey)
        if !hotkeys.start() {
            alert("Хоткей не запущен",
                  "Нет доступа Accessibility. Выдай доступ и перезапусти.",
                  openPane: .accessibility)
        }
    }

    private func handle(_ gesture: HotkeyGestureOutput) {
        switch gesture {
        case .startPushToTalk, .startToggle:
            startRecording()
        case .stopPushToTalk, .stopToggle:
            stopRecordingAndTranscribe()
        }
    }

    private func startRecording() {
        guard stateMachine.startRecording() else { hotkeys.resetGesture(); return }
        do { try audio.start(); overlay.show() }
        catch { stateMachine.reset(); alert("Микрофон недоступен", "\(error)", openPane: .microphone) }
    }

    private func stopRecordingAndTranscribe() {
        guard stateMachine.beginTranscribing() else { hotkeys.resetGesture(); return }
        let pcm = audio.stop()
        // Обрезаем тишину по краям — убирает «галлюцинации» на хвостовой тишине.
        let voiced = AudioMath.trimmedToVoice(pcm)
        // Слишком короткая/пустая запись (<0.3с) — игнорируем, чтобы не падать в движке.
        guard voiced.count >= 4800 else { stateMachine.finish(); overlay.hide(); return }
        // Оставляем оверлей видимым в режиме «обработки» до конца расшифровки.
        overlay.showProcessing()
        let lang = config.language
        Task {
            do {
                let raw = try await engine.transcribe(pcm16k: voiced, language: lang)
                let text = TranscriptCleanup.clean(raw)
                await MainActor.run {
                    self.overlay.hide()
                    if text.isEmpty {
                        self.stateMachine.finish()
                    } else {
                        self.history.add(text)
                        self.stateMachine.beginInserting()
                        self.injector.insert(text)
                        self.stateMachine.finish()
                    }
                }
            } catch {
                await MainActor.run {
                    self.overlay.hide()
                    self.stateMachine.reset()
                    self.alert("Ошибка распознавания", "\(error)", openPane: nil)
                }
            }
        }
    }

    private func alert(_ title: String, _ message: String, openPane: Permissions.SettingsPane?) {
        let a = NSAlert()
        a.messageText = title
        a.informativeText = message
        if let pane = openPane {
            a.addButton(withTitle: "Открыть настройки")
            a.addButton(withTitle: "Закрыть")
            if a.runModal() == .alertFirstButtonReturn { Permissions.openSystemSettings(pane) }
        } else {
            a.runModal()
        }
    }
}
