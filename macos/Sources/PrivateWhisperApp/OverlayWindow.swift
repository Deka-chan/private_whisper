import AppKit

/// Клик-сквозь, не активирующее окно с эквалайзером у нижнего края экрана.
final class OverlayWindow {
    private let panel: NSPanel
    private let waveView: EqualizerView

    init() {
        let size = NSSize(width: 240, height: 64)
        waveView = EqualizerView(frame: NSRect(origin: .zero, size: size))
        panel = NSPanel(contentRect: NSRect(origin: .zero, size: size),
                        styleMask: [.borderless, .nonactivatingPanel],
                        backing: .buffered, defer: false)
        panel.isFloatingPanel = true
        panel.level = .floating
        panel.backgroundColor = .clear
        panel.isOpaque = false
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary, .ignoresCycle]
        panel.contentView = waveView
        reposition()
    }

    private func reposition() {
        guard let screen = NSScreen.main else { return }
        let f = panel.frame
        let x = screen.frame.midX - f.width / 2
        let y = screen.frame.minY + 90
        panel.setFrameOrigin(NSPoint(x: x, y: y))
    }

    func show() {
        reposition()
        waveView.startAnimating()
        panel.alphaValue = 0
        panel.orderFrontRegardless()
        NSAnimationContext.runAnimationGroup { ctx in
            ctx.duration = 0.18
            panel.animator().alphaValue = 1
        }
    }

    /// Переключает оверлей в режим «идёт расшифровка» (окно остаётся видимым).
    func showProcessing() { waveView.setMode(.processing) }

    func hide() {
        NSAnimationContext.runAnimationGroup({ ctx in
            ctx.duration = 0.18
            panel.animator().alphaValue = 0
        }, completionHandler: { [weak self] in
            self?.waveView.stopAnimating()
            self?.panel.orderOut(nil)
        })
    }

    func push(level: Float) { waveView.push(level: CGFloat(level)) }
}

/// Эквалайзер: набор столбиков, реагирующих на громкость, с плавной анимацией,
/// градиентной заливкой и мягким свечением.
private final class EqualizerView: NSView {
    enum Mode { case listening, processing }

    private let barCount: Int
    private var heights: [CGFloat]          // текущие высоты (0...1)
    private var profile: [CGFloat]          // колоколообразный профиль (центр выше)
    private var inputEnergy: CGFloat = 0     // сглаженный уровень входа
    private var phase: CGFloat = 0           // счётчик кадров для лёгкого мерцания
    private var mode: Mode = .listening
    private var sweep: CGFloat = 0           // фаза «бегущей полосы» в режиме обработки
    private var timer: Timer?

    private let topColor = NSColor(srgbRed: 0.40, green: 0.80, blue: 1.0, alpha: 1)   // яркий голубой
    private let bottomColor = NSColor(srgbRed: 0.60, green: 0.40, blue: 1.0, alpha: 1) // фиолетовый

    override init(frame frameRect: NSRect) {
        let count = 19
        barCount = count
        heights = Array(repeating: 0, count: count)
        profile = (0..<count).map { i in
            let x = CGFloat(i) / CGFloat(count - 1)   // 0...1
            return 0.55 + 0.45 * sin(x * .pi)          // выше в центре
        }
        super.init(frame: frameRect)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    func push(level: CGFloat) {
        guard mode == .listening else { return }
        // Перцептивная шкала (sqrt): даже тихая речь заметно поднимает столбики.
        let amped = min(sqrt(max(level, 0)) * 2.8, 1)
        inputEnergy = max(inputEnergy, amped)
    }

    func setMode(_ newMode: Mode) {
        mode = newMode
        if newMode == .processing { sweep = 0; inputEnergy = 0 }
    }

    func startAnimating() {
        heights = Array(repeating: 0, count: barCount)
        inputEnergy = 0
        phase = 0
        mode = .listening
        timer?.invalidate()
        let t = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            self?.tick()
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    func stopAnimating() {
        timer?.invalidate()
        timer = nil
    }

    private func tick() {
        phase += 1
        if mode == .processing {
            // «Бегущая полоса» туда-сюда — индикатор обработки.
            sweep += 0.07
            let pos = (sin(sweep) * 0.5 + 0.5) * CGFloat(barCount - 1)
            for i in 0..<barCount {
                let band = max(0, 1 - abs(CGFloat(i) - pos) / 3.0)
                let target = 0.12 + 0.82 * band
                heights[i] += (target - heights[i]) * 0.35
            }
        } else {
            inputEnergy *= 0.90  // плавный спад при тишине
            for i in 0..<barCount {
                // Тонкая «бегущая» волна слева-направо, чтобы было живо в тишине.
                let idle = 0.07 * (0.5 + 0.5 * sin(phase * 0.16 - CGFloat(i) * 0.7))
                // Всплеск от голоса доминирует над фоновой волной.
                let voice = inputEnergy * profile[i] * (0.75 + 0.25 * sin(phase * 0.25 + CGFloat(i) * 0.8))
                let target = min(1, idle + voice * 1.15)
                heights[i] += (target - heights[i]) * 0.4
            }
        }
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current else { return }

        // Фон-капсула
        let bg = NSBezierPath(roundedRect: bounds, xRadius: 16, yRadius: 16)
        NSColor(white: 0.04, alpha: 0.62).setFill()
        bg.fill()

        let inset: CGFloat = 14
        let usableW = bounds.width - inset * 2
        let barW: CGFloat = 6
        let gap = (usableW - CGFloat(barCount) * barW) / CGFloat(barCount - 1)
        let maxH = bounds.height - 18
        let midY = bounds.midY

        // Союз всех столбиков как путь для клиппинга под градиент
        let barsPath = NSBezierPath()
        var x = inset
        for h in heights {
            let bh = max(4, h * maxH)
            let rect = NSRect(x: x, y: midY - bh / 2, width: barW, height: bh)
            barsPath.append(NSBezierPath(roundedRect: rect, xRadius: 3, yRadius: 3))
            x += barW + gap
        }

        ctx.saveGraphicsState()
        // Мягкое свечение
        let glow = NSShadow()
        glow.shadowColor = topColor.withAlphaComponent(0.65)
        glow.shadowBlurRadius = 9
        glow.shadowOffset = .zero
        glow.set()
        // Клип по столбикам и заливка вертикальным градиентом
        barsPath.addClip()
        let gradient = NSGradient(starting: bottomColor, ending: topColor)
        gradient?.draw(in: bounds, angle: 90)
        ctx.restoreGraphicsState()
    }
}
