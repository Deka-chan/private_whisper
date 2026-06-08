import Foundation

/// Убирает хвостовые «галлюцинации» Whisper — шаблонные фразы, которые модель
/// дописывает на тишине (например, «Thank you», «Спасибо за просмотр»).
public enum TranscriptCleanup {
    public static func clean(_ text: String) -> String {
        var t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let hasCyrillic = t.range(of: "\\p{Cyrillic}", options: .regularExpression) != nil

        // Почти всегда галлюцинации, режем независимо от языка.
        var phrases = ["спасибо за просмотр", "продолжение следует"]
        // Английские «спасибо», прилипающие к русской речи на хвосте.
        if hasCyrillic {
            phrases += ["thank you for watching", "thanks for watching",
                        "thank you very much", "thank you"]
        }

        var changed = true
        while changed {
            changed = false
            for p in phrases {
                // Перед фразой режем только пробелы — точку/знак предыдущего
                // предложения сохраняем; после фразы убираем её пунктуацию.
                let pattern = "(?i)\\s*" + NSRegularExpression.escapedPattern(for: p) + "[\\s.,!?\\-—]*$"
                if let r = t.range(of: pattern, options: .regularExpression) {
                    t.removeSubrange(r)
                    t = t.trimmingCharacters(in: .whitespacesAndNewlines)
                    changed = true
                }
            }
        }
        return t
    }
}
