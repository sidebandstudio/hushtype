import AppKit
final class RejectingView: NSView {
 override var acceptsFirstResponder: Bool { true }
 override func keyDown(with event: NSEvent) {}
 override func performKeyEquivalent(with event: NSEvent) -> Bool { true }
}
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let window = NSWindow(contentRect:NSRect(x:500,y:500,width:460,height:160), styleMask:[.titled,.closable], backing:.buffered, defer:false)
window.title = "hushtype paste rejection check"
let view = RejectingView(frame:window.contentView!.bounds)
window.contentView = view
let label = NSTextField(labelWithString:"This test window ignores keystrokes.")
label.frame = NSRect(x:32,y:65,width:400,height:24)
view.addSubview(label)
window.makeFirstResponder(view)
window.makeKeyAndOrderFront(nil)
app.activate(ignoringOtherApps:true)
app.run()
