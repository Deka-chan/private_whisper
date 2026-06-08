import XCTest
@testable import PrivateWhisperCore

final class ConfigTests: XCTestCase {
    private func freshDefaults() -> UserDefaults {
        let suite = "test-\(UUID().uuidString)"
        let d = UserDefaults(suiteName: suite)!
        d.removePersistentDomain(forName: suite)
        return d
    }

    func testDefaults() {
        let c = Config(defaults: freshDefaults())
        XCTAssertEqual(c.language, .auto)
        XCTAssertEqual(c.pasteRestoreDelay, 0.15, accuracy: 1e-6)
    }

    func testLanguageRoundTrip() {
        let d = freshDefaults()
        let c = Config(defaults: d)
        c.language = .ru
        let reloaded = Config(defaults: d)
        XCTAssertEqual(reloaded.language, .ru)
    }
}
