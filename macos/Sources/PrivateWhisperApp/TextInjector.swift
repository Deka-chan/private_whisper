import AppKit
import CoreGraphics

/// Вставляет текст по курсору, печатая его как синтезированные клавиатурные
/// события (Unicode). Буфер обмена не трогается вообще — поэтому ничего из него
/// не теряется и не вставляется по ошибке.
final class TextInjector {
    init() {}

    func insert(_ text: String) {
        guard !text.isEmpty else { return }
        let src = CGEventSource(stateID: .combinedSessionState)
        // keyboardSetUnicodeString надёжно отправляет короткие куски —
        // дробим текст по границам символов, чтобы ничего не потерялось.
        for chunk in text.chunkedByCharacters(maxUTF16: 18) {
            let utf16 = Array(chunk.utf16)
            let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true)
            let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false)
            utf16.withUnsafeBufferPointer { buf in
                down?.keyboardSetUnicodeString(stringLength: buf.count, unicodeString: buf.baseAddress)
                up?.keyboardSetUnicodeString(stringLength: buf.count, unicodeString: buf.baseAddress)
            }
            down?.post(tap: .cghidEventTap)
            up?.post(tap: .cghidEventTap)
        }
    }
}

private extension String {
    /// Дробит строку на куски не длиннее `maxUTF16` кодовых единиц UTF-16,
    /// не разрывая символы (суррогатные пары эмодзи и т.п.).
    func chunkedByCharacters(maxUTF16: Int) -> [String] {
        var chunks: [String] = []
        var current = ""
        var count = 0
        for ch in self {
            let n = String(ch).utf16.count
            if count + n > maxUTF16, !current.isEmpty {
                chunks.append(current); current = ""; count = 0
            }
            current.append(ch); count += n
        }
        if !current.isEmpty { chunks.append(current) }
        return chunks
    }
}
