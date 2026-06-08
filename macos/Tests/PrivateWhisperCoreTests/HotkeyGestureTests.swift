import XCTest
@testable import PrivateWhisperCore

final class HotkeyGestureTests: XCTestCase {
    private func makeRecognizer() -> (HotkeyGestureRecognizer, () -> [HotkeyGestureOutput]) {
        let r = HotkeyGestureRecognizer()
        var outputs: [HotkeyGestureOutput] = []
        r.onOutput = { outputs.append($0) }
        // Таймеры в тестах вызываем вручную, поэтому замыкания — no-op.
        r.requestHoldTimer = {}
        r.requestDoubleTapTimer = {}
        return (r, { outputs })
    }

    func testHoldStartsAndStopsPushToTalk() {
        let (r, outputs) = makeRecognizer()
        r.keyDown()
        r.holdTimerFired()            // клавиша всё ещё зажата -> push-to-talk
        r.keyUp()
        XCTAssertEqual(outputs(), [.startPushToTalk, .stopPushToTalk])
    }

    func testDoubleTapStartsToggleThenSinglePressStops() {
        let (r, outputs) = makeRecognizer()
        // первый тап
        r.keyDown(); r.keyUp()
        // второй тап в пределах окна -> toggle on
        r.keyDown(); r.keyUp()
        XCTAssertEqual(outputs(), [.startToggle])
        // одиночное нажатие останавливает toggle
        r.keyDown(); r.keyUp()
        XCTAssertEqual(outputs(), [.startToggle, .stopToggle])
    }

    func testLoneSingleTapDoesNothing() {
        let (r, outputs) = makeRecognizer()
        r.keyDown(); r.keyUp()
        r.doubleTapTimerFired()       // окно двойного нажатия истекло
        XCTAssertEqual(outputs(), [])
    }

    func testHoldNotConfusedWithToggle() {
        let (r, outputs) = makeRecognizer()
        r.keyDown(); r.holdTimerFired(); r.keyUp()  // push-to-talk цикл
        XCTAssertEqual(outputs(), [.startPushToTalk, .stopPushToTalk])
    }

    func testPressDuringToggleStopsImmediatelyEvenIfHeld() {
        let (r, outputs) = makeRecognizer()
        r.keyDown(); r.keyUp(); r.keyDown(); r.keyUp()   // toggle on
        r.keyDown()                                       // нажатие при активном toggle = стоп
        XCTAssertEqual(outputs(), [.startToggle, .stopToggle])
        r.keyUp()
        XCTAssertEqual(outputs(), [.startToggle, .stopToggle]) // keyUp ничего не добавляет
    }

    func testToggleOffThenSingleTapDoesNothing() {
        let (r, outputs) = makeRecognizer()
        r.keyDown(); r.keyUp(); r.keyDown(); r.keyUp()   // toggle on
        r.keyDown(); r.keyUp()                            // single press stops toggle
        // одиночный тап после остановки не должен ничего запускать
        r.keyDown(); r.keyUp()
        r.doubleTapTimerFired()
        XCTAssertEqual(outputs(), [.startToggle, .stopToggle])
    }

    func testToggleOffThenHoldStartsPushToTalk() {
        let (r, outputs) = makeRecognizer()
        r.keyDown(); r.keyUp(); r.keyDown(); r.keyUp()   // toggle on
        r.keyDown(); r.keyUp()                            // single press stops toggle
        // последующее удержание должно работать как push-to-talk
        r.keyDown(); r.holdTimerFired(); r.keyUp()
        XCTAssertEqual(outputs(), [.startToggle, .stopToggle, .startPushToTalk, .stopPushToTalk])
    }

    func testResetClearsState() {
        let (r, outputs) = makeRecognizer()
        // встаём в активный toggle
        r.keyDown(); r.keyUp(); r.keyDown(); r.keyUp()   // startToggle
        XCTAssertEqual(outputs(), [.startToggle])
        // сбрасываем — внутреннее состояние должно очиститься
        r.reset()
        // после reset двойной тап снова даёт startToggle (а не stopToggle)
        r.keyDown(); r.keyUp(); r.keyDown(); r.keyUp()
        XCTAssertEqual(outputs(), [.startToggle, .startToggle])
    }
}
