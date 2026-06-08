import Foundation

public enum HotkeyGestureOutput: Equatable, Sendable {
    case startPushToTalk
    case stopPushToTalk
    case startToggle
    case stopToggle
}

/// Распознаёт жесты одной клавиши: удержание (push-to-talk) и двойное нажатие (toggle).
/// Не владеет таймерами — просит хозяина запланировать их через замыкания.
public final class HotkeyGestureRecognizer {
    public var onOutput: ((HotkeyGestureOutput) -> Void)?
    /// Хозяин должен запланировать одноразовый таймер удержания, который вызовет holdTimerFired().
    public var requestHoldTimer: (() -> Void)?
    /// Хозяин должен запланировать одноразовый таймер окна двойного нажатия -> doubleTapTimerFired().
    public var requestDoubleTapTimer: (() -> Void)?

    private var keyIsDown = false
    private var pushToTalkActive = false
    private var toggleActive = false
    private var awaitingSecondTap = false
    private var holdResolved = false   // сработал ли таймер удержания для текущего нажатия

    public init() {}

    public func keyDown() {
        guard !keyIsDown else { return }   // защита от автоповтора
        keyIsDown = true
        holdResolved = false

        // Нажатие при активном toggle = немедленная остановка.
        if toggleActive {
            toggleActive = false
            holdResolved = true   // поглотить парный keyUp, чтобы он не считался первым тапом
            onOutput?(.stopToggle)
            return
        }
        // Иначе ждём: либо удержание (push-to-talk), либо отпускание (тап).
        requestHoldTimer?()
    }

    public func keyUp() {
        guard keyIsDown else { return }
        keyIsDown = false

        if pushToTalkActive {
            pushToTalkActive = false
            onOutput?(.stopPushToTalk)
            return
        }
        if holdResolved { return }  // удержание уже было обработано/отменено

        // Это тап (отпустили до порога удержания).
        if awaitingSecondTap {
            awaitingSecondTap = false
            toggleActive = true
            onOutput?(.startToggle)
        } else {
            awaitingSecondTap = true
            requestDoubleTapTimer?()
        }
    }

    /// Вызывается хозяином, когда истёк порог удержания.
    public func holdTimerFired() {
        guard keyIsDown, !pushToTalkActive, !toggleActive else { return }
        holdResolved = true
        pushToTalkActive = true
        onOutput?(.startPushToTalk)
    }

    /// Вызывается хозяином, когда истекло окно двойного нажатия.
    public func doubleTapTimerFired() {
        // Если второй тап не пришёл — это был одиночный тап, ничего не делаем.
        awaitingSecondTap = false
    }

    /// Полный сброс внутреннего состояния (например, когда жест не удалось
    /// обработать на стороне приложения и распознаватель надо ресинхронизировать).
    public func reset() {
        keyIsDown = false
        pushToTalkActive = false
        toggleActive = false
        awaitingSecondTap = false
        holdResolved = false
    }
}
