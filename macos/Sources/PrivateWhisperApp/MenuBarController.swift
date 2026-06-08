import AppKit
import PrivateWhisperCore

@MainActor final class MenuBarController: NSObject {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let loginItem = NSMenuItem(title: "Запускать при входе", action: nil, keyEquivalent: "")

    var onOpenSettings: (() -> Void)?
    var onOpenHistory: (() -> Void)?
    var onToggleLaunchAtLogin: (() -> Void)?
    var onQuit: (() -> Void)?

    override init() {
        super.init()
        setIcon(for: .idle)
        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "История диктовок…", action: #selector(history), keyEquivalent: "h"))
        menu.addItem(NSMenuItem(title: "Настройки…", action: #selector(settings), keyEquivalent: ","))
        loginItem.action = #selector(toggleLaunchAtLogin)
        menu.addItem(loginItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Выход", action: #selector(quit), keyEquivalent: "q"))
        for item in menu.items { item.target = self }
        statusItem.menu = menu
    }

    func setIcon(for state: AppState) {
        let symbol: String
        switch state {
        case .idle: symbol = "mic"
        case .recording: symbol = "mic.fill"
        case .transcribing: symbol = "waveform"
        case .inserting: symbol = "text.cursor"
        }
        statusItem.button?.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
    }

    /// Отмечает галочкой пункт «Запускать при входе».
    func setLaunchAtLogin(_ on: Bool) {
        loginItem.state = on ? .on : .off
    }

    /// Показывает короткий текст рядом с иконкой (например, прогресс загрузки).
    /// Передай nil, чтобы убрать текст.
    func setStatusText(_ text: String?) {
        statusItem.button?.title = text.map { " \($0)" } ?? ""
    }

    @objc private func settings() { onOpenSettings?() }
    @objc private func history() { onOpenHistory?() }
    @objc private func toggleLaunchAtLogin() { onToggleLaunchAtLogin?() }
    @objc private func quit() { onQuit?() }
}
