import Foundation

/// Единственный владелец перехода состояний приложения.
/// Методы возвращают true, если переход разрешён и выполнен.
public final class StateMachine {
    public private(set) var state: AppState = .idle
    public var onChange: ((AppState) -> Void)?

    public init() {}

    @discardableResult
    public func startRecording() -> Bool { transition(to: .recording, from: [.idle]) }

    @discardableResult
    public func beginTranscribing() -> Bool { transition(to: .transcribing, from: [.recording]) }

    @discardableResult
    public func beginInserting() -> Bool { transition(to: .inserting, from: [.transcribing]) }

    /// Завершить цикл -> idle. Разрешено из transcribing (пустой текст) и inserting.
    @discardableResult
    public func finish() -> Bool { transition(to: .idle, from: [.transcribing, .inserting]) }

    /// Принудительный сброс в idle из любого состояния (ошибка/отмена).
    public func reset() {
        guard state != .idle else { return }
        state = .idle
        onChange?(.idle)
    }

    private func transition(to new: AppState, from allowed: [AppState]) -> Bool {
        guard allowed.contains(state) else { return false }
        state = new
        onChange?(new)
        return true
    }
}
