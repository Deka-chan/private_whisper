import XCTest
@testable import PrivateWhisperCore

final class HistoryStoreTests: XCTestCase {
    private func freshDefaults() -> UserDefaults {
        let suite = "test-\(UUID().uuidString)"
        let d = UserDefaults(suiteName: suite)!
        d.removePersistentDomain(forName: suite)
        return d
    }

    func testAddPrependsAndCaps() {
        let store = HistoryStore(defaults: freshDefaults(), maxItems: 3)
        store.add("один"); store.add("два"); store.add("три"); store.add("четыре")
        XCTAssertEqual(store.items, ["четыре", "три", "два"]) // новейшее первым, обрезано до 3
    }

    func testIgnoresEmpty() {
        let store = HistoryStore(defaults: freshDefaults())
        store.add("   ")
        XCTAssertTrue(store.items.isEmpty)
    }

    func testUpdateAtIndex() {
        let d = freshDefaults()
        let store = HistoryStore(defaults: d)
        store.add("привет")
        store.update(at: 0, text: "привет, мир")
        XCTAssertEqual(store.items.first, "привет, мир")
    }

    func testPersistsAcrossInstances() {
        let d = freshDefaults()
        HistoryStore(defaults: d).add("сохранись")
        XCTAssertEqual(HistoryStore(defaults: d).items, ["сохранись"])
    }
}
