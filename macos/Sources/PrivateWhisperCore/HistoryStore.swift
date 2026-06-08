import Foundation

/// Хранит последние диктовки (по умолчанию 5), с сохранением между запусками.
public final class HistoryStore {
    private let defaults: UserDefaults
    private let key = "dictationHistory"
    private let maxItems: Int

    public init(defaults: UserDefaults = .standard, maxItems: Int = 5) {
        self.defaults = defaults
        self.maxItems = maxItems
    }

    public var items: [String] {
        defaults.stringArray(forKey: key) ?? []
    }

    /// Добавляет новую диктовку в начало списка, обрезая до maxItems.
    public func add(_ text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        var arr = items
        arr.insert(trimmed, at: 0)
        if arr.count > maxItems { arr = Array(arr.prefix(maxItems)) }
        defaults.set(arr, forKey: key)
    }

    /// Заменяет текст элемента по индексу (после редактирования в окне истории).
    public func update(at index: Int, text: String) {
        var arr = items
        guard arr.indices.contains(index) else { return }
        arr[index] = text
        defaults.set(arr, forKey: key)
    }
}
