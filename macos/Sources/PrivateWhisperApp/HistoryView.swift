import SwiftUI
import AppKit

struct HistoryView: View {
    @State var items: [String]
    let onCopy: (String) -> Void
    let onUpdate: (Int, String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Последние диктовки")
                .font(.headline)

            if items.isEmpty {
                Text("Пока пусто — продиктуй что-нибудь.")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 8)
            } else {
                ScrollView {
                    VStack(spacing: 14) {
                        ForEach(items.indices, id: \.self) { i in
                            VStack(alignment: .trailing, spacing: 6) {
                                TextEditor(text: Binding(
                                    get: { items[i] },
                                    set: { items[i] = $0; onUpdate(i, $0) }
                                ))
                                .font(.body)
                                .frame(height: 64)
                                .padding(6)
                                .background(Color(nsColor: .textBackgroundColor))
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                                .overlay(RoundedRectangle(cornerRadius: 8).stroke(.quaternary))

                                Button {
                                    onCopy(items[i])
                                } label: {
                                    Label("Копировать", systemImage: "doc.on.doc")
                                }
                                .buttonStyle(.bordered)
                                .controlSize(.small)
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .padding(20)
        .frame(width: 440, height: 460, alignment: .topLeading)
    }
}

/// Контроллер окна истории поверх SwiftUI-вью.
@MainActor
final class HistoryWindowController {
    private var window: NSWindow?

    func show(items: [String],
              onCopy: @escaping (String) -> Void,
              onUpdate: @escaping (Int, String) -> Void) {
        // Каждый раз пересоздаём, чтобы показать свежий список.
        window?.close()
        let view = HistoryView(items: items, onCopy: onCopy, onUpdate: onUpdate)
        let host = NSHostingController(rootView: view)
        let win = NSWindow(contentViewController: host)
        win.title = "История диктовок"
        win.styleMask = [.titled, .closable]
        win.isReleasedWhenClosed = false
        win.setContentSize(NSSize(width: 440, height: 460))
        self.window = win
        win.center()
        win.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
}
