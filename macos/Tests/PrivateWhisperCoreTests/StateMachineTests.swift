import XCTest
@testable import PrivateWhisperCore

final class StateMachineTests: XCTestCase {
    func testStartsIdle() {
        let sm = StateMachine()
        XCTAssertEqual(sm.state, .idle)
    }

    func testHappyPath() {
        let sm = StateMachine()
        XCTAssertTrue(sm.startRecording())
        XCTAssertEqual(sm.state, .recording)
        XCTAssertTrue(sm.beginTranscribing())
        XCTAssertEqual(sm.state, .transcribing)
        XCTAssertTrue(sm.beginInserting())
        XCTAssertEqual(sm.state, .inserting)
        XCTAssertTrue(sm.finish())
        XCTAssertEqual(sm.state, .idle)
    }

    func testEmptyTranscriptReturnsToIdle() {
        let sm = StateMachine()
        _ = sm.startRecording()
        _ = sm.beginTranscribing()
        XCTAssertTrue(sm.finish()) // пустой результат -> сразу idle
        XCTAssertEqual(sm.state, .idle)
    }

    func testCannotStartRecordingTwice() {
        let sm = StateMachine()
        XCTAssertTrue(sm.startRecording())
        XCTAssertFalse(sm.startRecording())
        XCTAssertEqual(sm.state, .recording)
    }

    func testResetFromAnyState() {
        let sm = StateMachine()
        _ = sm.startRecording()
        sm.reset()
        XCTAssertEqual(sm.state, .idle)
    }

    func testTransitionCallbackFires() {
        let sm = StateMachine()
        var seen: [AppState] = []
        sm.onChange = { seen.append($0) }
        _ = sm.startRecording()
        _ = sm.beginTranscribing()
        XCTAssertEqual(seen, [.recording, .transcribing])
    }
}
