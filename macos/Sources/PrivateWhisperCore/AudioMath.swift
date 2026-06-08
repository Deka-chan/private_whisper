import Foundation

public enum AudioMath {
    /// Среднеквадратичный уровень сигнала (0...1 для нормализованного входа).
    public static func rms(_ samples: [Float]) -> Float {
        guard !samples.isEmpty else { return 0 }
        let sumSq = samples.reduce(Float(0)) { $0 + $1 * $1 }
        return (sumSq / Float(samples.count)).squareRoot()
    }

    /// Обрезает тишину в начале и конце записи (по окну RMS), оставляя небольшой
    /// запас. Убирает хвостовую тишину — главную причину «галлюцинаций» Whisper
    /// (дописывания вроде «Thank you»). Возвращает пустой массив, если всё тихо.
    public static func trimmedToVoice(_ samples: [Float],
                                      sampleRate: Int = 16000,
                                      threshold: Float = 0.01) -> [Float] {
        guard !samples.isEmpty else { return samples }
        let window = max(1, sampleRate / 50)   // ~20 мс
        let pad = max(0, sampleRate / 10)      // ~100 мс запас по краям

        func loud(at start: Int) -> Bool {
            let end = min(start + window, samples.count)
            var sum: Float = 0
            var i = start
            while i < end { sum += samples[i] * samples[i]; i += 1 }
            let n = max(1, end - start)
            return (sum / Float(n)).squareRoot() > threshold
        }

        var firstLoud = -1
        var s = 0
        while s < samples.count { if loud(at: s) { firstLoud = s; break }; s += window }
        guard firstLoud >= 0 else { return [] }   // всё тихо

        var lastLoud = samples.count
        var e = samples.count - window
        while e > firstLoud { if loud(at: e) { lastLoud = min(samples.count, e + window); break }; e -= window }

        let lo = max(0, firstLoud - pad)
        let hi = min(samples.count, lastLoud + pad)
        return Array(samples[lo..<hi])
    }

    /// Сводит interleaved-аудио к моно усреднением каналов.
    public static func downmixToMono(interleaved: [Float], channels: Int) -> [Float] {
        guard channels > 1 else { return interleaved }
        let frames = interleaved.count / channels
        var mono = [Float](repeating: 0, count: frames)
        for f in 0..<frames {
            var acc: Float = 0
            for c in 0..<channels { acc += interleaved[f * channels + c] }
            mono[f] = acc / Float(channels)
        }
        return mono
    }
}
