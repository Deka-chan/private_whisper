import Foundation
import ServiceManagement

/// Управление автозапуском приложения при входе в систему (macOS 13+).
enum LoginItem {
    static var isEnabled: Bool {
        SMAppService.mainApp.status == .enabled
    }

    /// Включает/выключает автозапуск. Возвращает фактическое состояние после операции.
    @discardableResult
    static func setEnabled(_ on: Bool) -> Bool {
        do {
            if on {
                if SMAppService.mainApp.status != .enabled {
                    try SMAppService.mainApp.register()
                }
            } else {
                try SMAppService.mainApp.unregister()
            }
        } catch {
            NSLog("LoginItem: не удалось изменить автозапуск: \(error)")
        }
        return isEnabled
    }
}
