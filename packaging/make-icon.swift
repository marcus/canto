// Renders Canto's app icon: the accent gradient with three sound waves.
// Usage: swift packaging/make-icon.swift out.png
import AppKit

let size: CGFloat = 1024
let image = NSImage(size: NSSize(width: size, height: size))
image.lockFocus()
let ctx = NSGraphicsContext.current!.cgContext
let inset: CGFloat = 100
let rect = CGRect(x: inset, y: inset, width: size - 2 * inset, height: size - 2 * inset)
let path = CGPath(roundedRect: rect, cornerWidth: 185, cornerHeight: 185, transform: nil)
ctx.addPath(path)
ctx.clip()
let colors = [NSColor(red: 1.0, green: 0.31, blue: 0.545, alpha: 1).cgColor,
              NSColor(red: 1.0, green: 0.635, blue: 0.30, alpha: 1).cgColor] as CFArray
let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors, locations: [0, 1])!
ctx.drawLinearGradient(gradient, start: CGPoint(x: rect.minX, y: rect.maxY), end: CGPoint(x: rect.maxX, y: rect.minY), options: [])
ctx.setStrokeColor(NSColor(white: 1, alpha: 0.92).cgColor)
ctx.setLineWidth(46)
ctx.setLineCap(.round)
for (i, offset) in [-150.0, 0.0, 150.0].enumerated() {
    let y = size / 2 + CGFloat(offset)
    let amp: CGFloat = i == 1 ? 70 : 48
    let wave = CGMutablePath()
    let x0 = rect.minX + 150, x1 = rect.maxX - 150
    wave.move(to: CGPoint(x: x0, y: y))
    let steps = 120
    for s in 1...steps {
        let t = CGFloat(s) / CGFloat(steps)
        let x = x0 + (x1 - x0) * t
        wave.addLine(to: CGPoint(x: x, y: y + amp * sin(t * .pi * 4)))
    }
    ctx.addPath(wave)
    ctx.strokePath()
}
image.unlockFocus()
let rep = NSBitmapImageRep(data: image.tiffRepresentation!)!
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
