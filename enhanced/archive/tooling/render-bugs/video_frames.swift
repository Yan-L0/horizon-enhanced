import Foundation
import AVFoundation
import AppKit
for (index, path) in CommandLine.arguments.dropFirst().enumerated() {
    let asset = AVURLAsset(url: URL(fileURLWithPath: path))
    let duration = CMTimeGetSeconds(asset.duration)
    print("video \(index): \(duration) seconds")
    let generator = AVAssetImageGenerator(asset: asset)
    generator.appliesPreferredTrackTransform = true
    generator.maximumSize = CGSize(width: 420, height: 230)
    let sheet = NSImage(size: NSSize(width: 1260, height: 780))
    sheet.lockFocus()
    NSColor.darkGray.setFill(); NSRect(x: 0, y: 0, width: 1260, height: 780).fill()
    for i in 0..<9 {
        let seconds = duration * (Double(i) + 0.2) / 9.0
        do {
            let cg = try generator.copyCGImage(at: CMTime(seconds: seconds, preferredTimescale: 600), actualTime: nil)
            let im = NSImage(cgImage: cg, size: NSSize(width: cg.width, height: cg.height))
            let x = (i % 3) * 420, y = (2 - i / 3) * 260
            im.draw(in: NSRect(x: x, y: y, width: 420, height: 230))
            NSString(format: "%.2f s", seconds).draw(at: NSPoint(x: x + 5, y: y + 234), withAttributes: [.foregroundColor: NSColor.white, .font: NSFont.systemFont(ofSize: 16)])
        } catch { print(error) }
    }
    sheet.unlockFocus()
    let bitmap = NSBitmapImageRep(data: sheet.tiffRepresentation!)!
    try bitmap.representation(using: .jpeg, properties: [.compressionFactor: 0.85])!.write(to: URL(fileURLWithPath: "work/render-bugs/video-\(index)-sheet.jpg"))
}
