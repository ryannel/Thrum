// Render the same specimen strings through macOS CoreText for visual comparison.
import CoreGraphics
import CoreText
import Foundation
import ImageIO
import UniformTypeIdentifiers

let faceURL = URL(fileURLWithPath: CommandLine.arguments[1])
let face = CGFont(CGDataProvider(url: faceURL as CFURL)!)!
let scale = Int(CommandLine.arguments[2])!
let destination = URL(fileURLWithPath: CommandLine.arguments[3])
let width = 490 * scale
let height = 335 * scale
let colour = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                        bytesPerRow: 0, space: colour,
                        bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
context.setFillColor(red: 0.13, green: 0.17, blue: 0.22, alpha: 1)
context.fill(CGRect(x: 0, y: 0, width: width, height: height))
context.setFillColor(red: 0.95, green: 0.96, blue: 0.96, alpha: 1)
context.fill(CGRect(x: 0, y: (335 - 286) * scale, width: width, height: 143 * scale))

let light = CGColor(red: 1, green: 1, blue: 1, alpha: 1)
let dark = CGColor(red: 0.13, green: 0.17, blue: 0.22, alpha: 1)

func line(_ text: String, points: Int, top: Int, colour: CGColor, outline: Bool = false) {
    let font = CTFontCreateWithGraphicsFont(face, CGFloat(points * scale), nil, nil)
    let attributes: [NSAttributedString.Key: Any] = [
        NSAttributedString.Key(kCTFontAttributeName as String): font,
        NSAttributedString.Key(kCTForegroundColorAttributeName as String): colour
    ]
    let baseline = CGPoint(x: 14 * scale,
        y: (335 - top) * scale - Int(ceil(CTFontGetAscent(font))))
    if outline {
        var stroke = attributes
        stroke[NSAttributedString.Key(kCTStrokeColorAttributeName as String)] = dark
        stroke[NSAttributedString.Key(kCTStrokeWidthAttributeName as String)] = 12.5
        let edge = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: stroke))
        context.textPosition = baseline
        CTLineDraw(edge, context)
    }
    let fill = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attributes))
    context.textPosition = baseline
    CTLineDraw(fill, context)
}

let pangram = "Sphinx of black quartz, judge my vow."
line(pangram, points: 12, top: 12, colour: light)
line(pangram, points: 16, top: 32, colour: light)
line(pangram, points: 24, top: 56, colour: light)
line("AV To r. · 0123456789 — “…” (€ × −)", points: 12, top: 91, colour: light)
line("AV To r. · 0123456789 — “…” (€ × −)", points: 12, top: 153, colour: dark)
line("Élève français · Grüße aus Köln", points: 16, top: 172, colour: dark)
line("Zażółć gęślą jaźń", points: 24, top: 196, colour: dark)
line("Příliš žluťoučký kůň úpěl ďábelské ódy.", points: 16, top: 232, colour: dark)
line("AV To r. · À Noël, déjà l’hiver.", points: 16, top: 303, colour: light, outline: true)

let target = CGImageDestinationCreateWithURL(destination as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(target, context.makeImage()!, nil)
precondition(CGImageDestinationFinalize(target))
