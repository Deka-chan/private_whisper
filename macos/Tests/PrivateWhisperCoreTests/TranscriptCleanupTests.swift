import XCTest
@testable import PrivateWhisperCore

final class TranscriptCleanupTests: XCTestCase {
    func testStripsTrailingEnglishThankYouAfterRussian() {
        XCTAssertEqual(
            TranscriptCleanup.clean("Спасибо тебе, Макс. Thank you."),
            "Спасибо тебе, Макс."
        )
    }

    func testKeepsLegitimateRussianSpasibo() {
        XCTAssertEqual(
            TranscriptCleanup.clean("Спасибо тебе большое."),
            "Спасибо тебе большое."
        )
    }

    func testDoesNotTouchEnglishOnlyThankYou() {
        // Без кириллицы «thank you» может быть легитимным — не трогаем.
        XCTAssertEqual(
            TranscriptCleanup.clean("I really want to thank you."),
            "I really want to thank you."
        )
    }

    func testStripsRussianHallucination() {
        XCTAssertEqual(
            TranscriptCleanup.clean("Это проверка. Спасибо за просмотр!"),
            "Это проверка."
        )
    }

    func testTrimsWhitespace() {
        XCTAssertEqual(TranscriptCleanup.clean("  привет  "), "привет")
    }
}
