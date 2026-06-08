import Foundation
import WhisperKit
import PrivateWhisperCore

enum WhisperEngineError: Error, Sendable {
    case notLoaded
}

/// Движок распознавания на WhisperKit (CoreML/Neural Engine).
/// Модель large-v3-turbo: быстро и точно на Apple Silicon. WhisperKit сам
/// скачивает и компилирует модель при первой загрузке.
final class WhisperKitEngine: TranscriptionEngine {
    private var pipe: WhisperKit?
    private let modelName: String

    init(modelName: String = "large-v3-v20240930_turbo") {
        self.modelName = modelName
    }

    func loadModel() async throws {
        guard pipe == nil else { return }
        let config = WhisperKitConfig(model: modelName)
        pipe = try await WhisperKit(config)
    }

    func transcribe(pcm16k: [Float], language: LanguageMode) async throws -> String {
        guard let pipe else { throw WhisperEngineError.notLoaded }
        // Для «Авто» явно включаем детекцию языка — иначе WhisperKit считает речь
        // английской по умолчанию. Для ru/en фиксируем язык напрямую.
        let options: DecodingOptions
        switch language {
        case .auto: options = DecodingOptions(task: .transcribe, language: nil, detectLanguage: true)
        case .ru:   options = DecodingOptions(task: .transcribe, language: "ru")
        case .en:   options = DecodingOptions(task: .transcribe, language: "en")
        }
        let results = try await pipe.transcribe(audioArray: pcm16k, decodeOptions: options)
        return results.map { $0.text }.joined(separator: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
