// sfai.app — native macOS shell: WKWebView window + embedded Decide-Act runtime.
// Launches `sfai serve --web <frontend>` as a child process (real engine, real
// models via provider keys), then opens the conversation workbench in a window.
import Cocoa
import WebKit

let resources = Bundle.main.resourceURL!
let backendBin = resources.appendingPathComponent("sfai")
let webDir = resources.appendingPathComponent("web")
let port: UInt16 = 8787
let baseURL = URL(string: "http://127.0.0.1:\(port)/")!

// ---- start the real backend ----
let child = Process()
child.executableURL = backendBin
child.arguments = ["serve", "--web", webDir.path, "--port", String(port)]
// Provider keys: GUI launches have a minimal environment; merge ~/.sfai/env if present.
var env = ProcessInfo.processInfo.environment
if let extra = try? String(contentsOfFile: (NSHomeDirectory() as NSString).appendingPathComponent(".sfai/env"), encoding: .utf8) {
    for line in extra.split(separator: "\n") {
        let parts = line.split(separator: "=", maxSplits: 1).map(String.init)
        if parts.count == 2, !parts[0].hasPrefix("#") { env[parts[0]] = parts[1] }
    }
}
child.environment = env
child.standardOutput = FileHandle.nullDevice
child.standardError = FileHandle.nullDevice
try? child.run()

// ---- app ----
let app = NSApplication.shared
app.setActivationPolicy(.regular)

final class AppDelegate: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    var web: WKWebView!

    func applicationDidFinishLaunching(_ notification: Notification) {
        let config = WKWebViewConfiguration()
        let inject = WKUserScript(
            source: "window.__SFAI_BACKEND__ = 'http://127.0.0.1:\(port)';",
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        )
        config.userContentController.addUserScript(inject)
        web = WKWebView(frame: .zero, configuration: config)
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1280, height: 840),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "System for future ai"
        window.center()
        window.contentView = web
        window.makeKeyAndOrderFront(nil)
        app.activate(ignoringOtherApps: true)
        waitForBackendThenLoad(attempts: 60)
    }

    func waitForBackendThenLoad(attempts: Int) {
        guard attempts > 0 else { web.load(URLRequest(url: baseURL)); return }
        var req = URLRequest(url: baseURL.appendingPathComponent("api/health"))
        req.timeoutInterval = 1.0
        URLSession.shared.dataTask(with: req) { [weak self] _, response, _ in
            let ok = (response as? HTTPURLResponse)?.statusCode == 200
            DispatchQueue.main.async {
                if ok { self?.web.load(URLRequest(url: baseURL)) }
                else { self?.waitForBackendThenLoad(attempts: attempts - 1) }
            }
        }.resume()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ notification: Notification) {
        if child.isRunning { child.terminate() }
    }
}

let delegate = AppDelegate()
app.delegate = delegate
app.run()
