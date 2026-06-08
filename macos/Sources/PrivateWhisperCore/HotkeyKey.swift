/// Клавиша-триггер для голосового ввода (выбирается в настройках).
public enum HotkeyKey: String, CaseIterable, Sendable {
    case fn
    case rightCommand
    case rightOption
    case rightControl

    public var displayName: String {
        switch self {
        case .fn: return "fn (🌐)"
        case .rightCommand: return "Правый ⌘"
        case .rightOption: return "Правый ⌥"
        case .rightControl: return "Правый ⌃"
        }
    }
}
