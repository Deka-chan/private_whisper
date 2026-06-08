import AppKit
import CoreGraphics
import PrivateWhisperCore

/// Глобальный перехват выбранной клавиши через CGEvent tap.
/// Модификаторы приходят как .flagsChanged; отслеживаем фронт/спад нужной клавиши.
final class HotkeyManager {
    private let recognizer = HotkeyGestureRecognizer()
    private var tap: CFMachPort?
    private var runLoopSource: CFRunLoopSource?
    private var keyWasDown = false
    private var key: HotkeyKey = .fn
    private var holdTimer: Timer?
    private var doubleTapTimer: Timer?

    private let holdThreshold: TimeInterval = 0.25
    private let doubleTapWindow: TimeInterval = 0.30

    var onGesture: ((HotkeyGestureOutput) -> Void)?

    init() {
        recognizer.onOutput = { [weak self] in self?.onGesture?($0) }
        recognizer.requestHoldTimer = { [weak self] in
            guard let self else { return }
            self.holdTimer?.invalidate()
            self.holdTimer = Timer.scheduledTimer(withTimeInterval: self.holdThreshold, repeats: false) { _ in
                self.recognizer.holdTimerFired()
            }
        }
        recognizer.requestDoubleTapTimer = { [weak self] in
            guard let self else { return }
            self.doubleTapTimer?.invalidate()
            self.doubleTapTimer = Timer.scheduledTimer(withTimeInterval: self.doubleTapWindow, repeats: false) { _ in
                self.recognizer.doubleTapTimerFired()
            }
        }
    }

    /// Меняет активную клавишу-триггер на лету (tap пересоздавать не нужно).
    func setKey(_ newKey: HotkeyKey) {
        key = newKey
        keyWasDown = false
        resetGesture()
    }

    /// Сбрасывает распознаватель и отменяет висящие таймеры (ресинхронизация).
    func resetGesture() {
        holdTimer?.invalidate()
        doubleTapTimer?.invalidate()
        recognizer.reset()
    }

    /// Возвращает false, если нет доступа Accessibility (tap создать нельзя).
    @discardableResult
    func start() -> Bool {
        let mask = (1 << CGEventType.flagsChanged.rawValue)
        let callback: CGEventTapCallBack = { _, _, event, refcon in
            let manager = Unmanaged<HotkeyManager>.fromOpaque(refcon!).takeUnretainedValue()
            manager.handle(event)
            return Unmanaged.passUnretained(event)
        }
        guard let tap = CGEvent.tapCreate(
            tap: .cgSessionEventTap, place: .headInsertEventTap,
            options: .listenOnly, eventsOfInterest: CGEventMask(mask),
            callback: callback, userInfo: Unmanaged.passUnretained(self).toOpaque()
        ) else { return false }

        self.tap = tap
        runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
        CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        return true
    }

    private func handle(_ event: CGEvent) {
        let down: Bool
        switch key {
        case .fn:
            // fn выражается отдельным флагом; различать левый/правый не нужно.
            down = event.flags.contains(.maskSecondaryFn)
        default:
            // Для правых модификаторов фильтруем по keycode конкретной клавиши,
            // состояние (нажата/отпущена) — по наличию соответствующего флага.
            let kc = event.getIntegerValueField(.keyboardEventKeycode)
            guard kc == key.keyCode else { return }
            down = event.flags.contains(key.mask)
        }
        if down && !keyWasDown {
            keyWasDown = true
            recognizer.keyDown()
        } else if !down && keyWasDown {
            keyWasDown = false
            recognizer.keyUp()
        }
    }
}

private extension HotkeyKey {
    /// Виртуальный keycode правой клавиши (для fn не используется).
    var keyCode: Int64 {
        switch self {
        case .fn: return 63           // kVK_Function
        case .rightCommand: return 54 // kVK_RightCommand
        case .rightOption: return 61  // kVK_RightOption
        case .rightControl: return 62 // kVK_RightControl
        }
    }

    var mask: CGEventFlags {
        switch self {
        case .fn: return .maskSecondaryFn
        case .rightCommand: return .maskCommand
        case .rightOption: return .maskAlternate
        case .rightControl: return .maskControl
        }
    }
}
