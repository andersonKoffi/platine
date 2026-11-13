import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private static let defaultSize = NSSize(width: 1280, height: 800)
  private static let minimumSize = NSSize(width: 960, height: 640)

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    contentViewController = flutterViewController
    title = "Platine"
    minSize = MainFlutterWindow.minimumSize
    setContentSize(MainFlutterWindow.defaultSize)
    center()

    RegisterGeneratedPlugins(registry: flutterViewController)

    super.awakeFromNib()
  }
}
