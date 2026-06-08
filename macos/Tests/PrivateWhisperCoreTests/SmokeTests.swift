import XCTest
@testable import PrivateWhisperCore

final class SmokeTests: XCTestCase {
    func testLanguageWhisperCodes() {
        XCTAssertEqual(LanguageMode.auto.whisperCode, "auto")
        XCTAssertEqual(LanguageMode.ru.whisperCode, "ru")
        XCTAssertEqual(LanguageMode.en.whisperCode, "en")
    }

    func testAllLanguageCases() {
        XCTAssertEqual(LanguageMode.allCases.count, 3)
    }
}
