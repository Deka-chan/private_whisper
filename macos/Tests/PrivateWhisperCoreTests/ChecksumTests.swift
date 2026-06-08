import XCTest
@testable import PrivateWhisperCore

final class ChecksumTests: XCTestCase {
    func testSHA256OfEmpty() {
        let data = Data()
        XCTAssertEqual(Checksum.sha256Hex(data),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    }

    func testSHA256OfAbc() {
        let data = Data("abc".utf8)
        XCTAssertEqual(Checksum.sha256Hex(data),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    }
}
