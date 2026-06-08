import AVFoundation
import PrivateWhisperCore

/// Захватывает микрофон, конвертирует в 16кГц моно Float32, накапливает кадры
/// и отдаёт уровень громкости для оверлея.
final class AudioCapture {
    private let engine = AVAudioEngine()
    private var converter: AVAudioConverter?
    private let targetFormat = AVAudioFormat(commonFormat: .pcmFormatFloat32,
                                             sampleRate: 16000, channels: 1, interleaved: false)!
    private var frames: [Float] = []
    private let lock = NSLock()

    /// Вызывается с RMS-уровнем (0...1) на каждый буфер — для волны.
    var onLevel: ((Float) -> Void)?

    func start() throws {
        frames.removeAll(keepingCapacity: true)
        let input = engine.inputNode
        let inputFormat = input.inputFormat(forBus: 0)
        converter = AVAudioConverter(from: inputFormat, to: targetFormat)

        input.installTap(onBus: 0, bufferSize: 4096, format: inputFormat) { [weak self] buffer, _ in
            self?.process(buffer)
        }
        engine.prepare()
        try engine.start()
    }

    /// Останавливает захват и возвращает накопленный моно-PCM 16 кГц.
    func stop() -> [Float] {
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        lock.lock(); defer { lock.unlock() }
        return frames
    }

    private func process(_ buffer: AVAudioPCMBuffer) {
        guard let converter else { return }
        let ratio = targetFormat.sampleRate / buffer.format.sampleRate
        let capacity = AVAudioFrameCount(Double(buffer.frameLength) * ratio + 1024)
        guard let out = AVAudioPCMBuffer(pcmFormat: targetFormat, frameCapacity: capacity) else { return }

        var provided = false
        converter.convert(to: out, error: nil) { _, status in
            if provided { status.pointee = .noDataNow; return nil }
            provided = true; status.pointee = .haveData; return buffer
        }
        guard let ptr = out.floatChannelData?[0] else { return }
        let chunk = Array(UnsafeBufferPointer(start: ptr, count: Int(out.frameLength)))

        onLevel?(AudioMath.rms(chunk))
        lock.lock(); frames.append(contentsOf: chunk); lock.unlock()
    }
}
