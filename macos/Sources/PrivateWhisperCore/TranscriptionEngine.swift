public protocol TranscriptionEngine: AnyObject {
    /// Загрузить модель в память. Бросает, если файл модели недоступен/битый.
    func loadModel() async throws
    /// Расшифровать моно PCM 16 кГц (Float32, [-1, 1]).
    func transcribe(pcm16k: [Float], language: LanguageMode) async throws -> String
}
