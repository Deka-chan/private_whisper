import XCTest
@testable import PrivateWhisperCore

final class ModelCatalogTests: XCTestCase {
    func testDefaultModelIsLargeV2() {
        let m = ModelCatalog.default
        XCTAssertEqual(m.fileName, "ggml-large-v2.bin")
        XCTAssertTrue(m.downloadURL.absoluteString.hasSuffix("ggml-large-v2.bin"))
        XCTAssertEqual(m.downloadURL.scheme, "https")
    }

    func testMinimumBytesIsReasonable() {
        // large-v2 ~3 ГБ; нижняя граница отсекает битые/обрезанные загрузки.
        XCTAssertGreaterThanOrEqual(ModelCatalog.default.minimumBytes, 2_500_000_000)
    }

    func testValidatesBySize() {
        let m = ModelCatalog.default
        XCTAssertFalse(m.isPlausible(byteCount: 1024))
        XCTAssertTrue(m.isPlausible(byteCount: 3_200_000_000))
    }
}
