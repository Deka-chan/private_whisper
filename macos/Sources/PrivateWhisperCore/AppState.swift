public enum AppState: Equatable, Sendable {
    case idle
    case recording
    case transcribing
    case inserting
}
