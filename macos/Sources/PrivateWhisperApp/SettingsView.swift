import SwiftUI
import AppKit
import PrivateWhisperCore

struct SettingsView: View {
    @State var language: LanguageMode
    @State var hotkey: HotkeyKey
    let onLanguageChange: (LanguageMode) -> Void
    let onHotkeyChange: (HotkeyKey) -> Void

    var body: some View {
        Form {
            Picker("Язык", selection: $language) {
                ForEach(LanguageMode.allCases, id: \.self) { mode in
                    Text(mode.displayName).tag(mode)
                }
            }
            .onChange(of: language) { _, new in onLanguageChange(new) }

            Picker("Клавиша", selection: $hotkey) {
                ForEach(HotkeyKey.allCases, id: \.self) { key in
                    Text(key.displayName).tag(key)
                }
            }
            .onChange(of: hotkey) { _, new in onHotkeyChange(new) }

            Text("Удержание — диктовка, пока зажата. Двойное нажатие — режим до повторного нажатия.")
                .font(.footnote).foregroundStyle(.secondary)
        }
        .padding(20)
        .frame(width: 380)
    }
}

/// Контроллер окна настроек поверх SwiftUI-вью.
@MainActor
final class SettingsWindowController {
    private var window: NSWindow?

    func show(language: LanguageMode,
              hotkey: HotkeyKey,
              onLanguageChange: @escaping (LanguageMode) -> Void,
              onHotkeyChange: @escaping (HotkeyKey) -> Void) {
        if let window { window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true); return }
        let view = SettingsView(language: language,
                                hotkey: hotkey,
                                onLanguageChange: onLanguageChange,
                                onHotkeyChange: onHotkeyChange)
        let host = NSHostingController(rootView: view)
        let win = NSWindow(contentViewController: host)
        win.title = "PrivateWhisper"
        win.styleMask = [.titled, .closable]
        win.isReleasedWhenClosed = false
        self.window = win
        win.center()
        win.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
}
