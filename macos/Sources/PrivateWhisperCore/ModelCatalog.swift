import Foundation

public struct ModelCatalog: Sendable {
    public let fileName: String
    public let downloadURL: URL
    public let minimumBytes: Int
    public let expectedSHA256: String?

    public func isPlausible(byteCount: Int) -> Bool {
        byteCount >= minimumBytes
    }

    // Модель серии 80-mel (large-v2): совместима с whisper.cpp, вшитым в SwiftWhisper,
    // и заметно точнее medium (особенно на русском).
    // large-v3-turbo использует 128 mel и требует более новой версии whisper.cpp.
    public static let `default` = ModelCatalog(
        fileName: "ggml-large-v2.bin",
        downloadURL: URL(string: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v2.bin")!,
        minimumBytes: 2_500_000_000,
        expectedSHA256: nil
    )
}
