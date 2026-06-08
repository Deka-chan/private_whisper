import Foundation

public final class Config {
    private let defaults: UserDefaults
    private enum Key {
        static let language = "language"
        static let pasteRestoreDelay = "pasteRestoreDelay"
        static let hotkey = "hotkey"
    }

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        if defaults.object(forKey: Key.pasteRestoreDelay) == nil {
            defaults.set(0.15, forKey: Key.pasteRestoreDelay)
        }
    }

    public var language: LanguageMode {
        get { LanguageMode(rawValue: defaults.string(forKey: Key.language) ?? "") ?? .auto }
        set { defaults.set(newValue.rawValue, forKey: Key.language) }
    }

    public var pasteRestoreDelay: TimeInterval {
        get { defaults.double(forKey: Key.pasteRestoreDelay) }
        set { defaults.set(newValue, forKey: Key.pasteRestoreDelay) }
    }

    public var hotkey: HotkeyKey {
        get { HotkeyKey(rawValue: defaults.string(forKey: Key.hotkey) ?? "") ?? .fn }
        set { defaults.set(newValue.rawValue, forKey: Key.hotkey) }
    }
}
