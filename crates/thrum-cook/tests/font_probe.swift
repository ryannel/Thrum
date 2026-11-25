// CoreText is an independent reader of the original TrueType font.
import CoreGraphics
import CoreText
import Foundation

let path = URL(fileURLWithPath: CommandLine.arguments[1])
let data = CGDataProvider(url: path as CFURL)!
let face = CGFont(data)!
let scalars = Array(32...126) + Array(160...383) +
    "–—‘’“”…•×−€←↑→↓".unicodeScalars.map { Int($0.value) }

var strikes = [[String: Any]]()
for points in [12, 16, 24] {
    for scale in [1, 2, 3] {
        let font = CTFontCreateWithGraphicsFont(face, CGFloat(points * scale), nil, nil)
        var glyphs = [String: [CGFloat]]()
        for code in Set(scalars) {
            var character = UniChar(code)
            var glyph = CGGlyph(0)
            guard CTFontGetGlyphsForCharacters(font, &character, &glyph, 1) else { continue }
            let advance = CTFontGetAdvancesForGlyphs(font, .horizontal, &glyph, nil, 1)
            let bounds = CTFontCreatePathForGlyph(font, glyph, nil)?.boundingBoxOfPath ?? .zero
            glyphs[String(code)] = bounds.isNull ? [advance, 0, 0, 0, 0] : [advance, bounds.minX, bounds.minY, bounds.maxX, bounds.maxY]
        }
        let kern: [(String, Double)] = ["AV", "To", "r."].map { pair in
            let line = CTLineCreateWithAttributedString(NSAttributedString(
                string: pair, attributes: [NSAttributedString.Key(kCTFontAttributeName as String): font]))
            let width = CTLineGetTypographicBounds(line, nil, nil, nil)
            let raw = pair.unicodeScalars.reduce(0.0) { $0 + Double(glyphs[String($1.value)]![0]) }
            return (pair, width - raw)
        }
        strikes.append([
            "points": points, "scale": scale, "glyphs": glyphs, "kern": Dictionary(uniqueKeysWithValues: kern),
            "line": [CTFontGetAscent(font), CTFontGetDescent(font), CTFontGetLeading(font)]
        ])
    }
}
FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: strikes, options: [.sortedKeys]))
