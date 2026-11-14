import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private static let defaultSize = NSSize(width: 1440, height: 900)
  private static let minimumSize = NSSize(width: 960, height: 640)

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    contentViewController = flutterViewController
    title = "Platine"
    minSize = MainFlutterWindow.minimumSize
    setContentSize(MainFlutterWindow.defaultSize)
    center()

    RegisterGeneratedPlugins(registry: flutterViewController)
    registerFolderPicker(messenger: flutterViewController.engine.binaryMessenger)

    super.awakeFromNib()
  }

  /// `platine/folder_picker` → `pickFolder`: native folder chooser.
  /// Choosing a folder grants the sandboxed app read access to it.
  private func registerFolderPicker(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: "platine/folder_picker", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard call.method == "pickFolder" else {
        result(FlutterMethodNotImplemented)
        return
      }
      let panel = NSOpenPanel()
      panel.canChooseDirectories = true
      panel.canChooseFiles = false
      panel.allowsMultipleSelection = false
      panel.prompt = "Choisir"
      panel.message = "Choisis un dossier de musique"
      let completion: (NSApplication.ModalResponse) -> Void = { response in
        result(response == .OK ? panel.url?.path : nil)
      }
      if let window = self {
        panel.beginSheetModal(for: window, completionHandler: completion)
      } else {
        completion(panel.runModal())
      }
    }
  }
}
