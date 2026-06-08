public enum LanguageMode: String, CaseIterable, Sendable {
    case auto
    case ru
    case en

    /// Код языка для whisper ("auto" -> авто-определение).
    public var whisperCode: String {
        switch self {
        case .auto: return "auto"
        case .ru: return "ru"
        case .en: return "en"
        }
    }

    public var displayName: String {
        switch self {
        case .auto: return "Авто"
        case .ru: return "Русский"
        case .en: return "English"
        }
    }
}
