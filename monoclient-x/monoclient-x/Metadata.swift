//
//  Item.swift
//  monoclient-x
//
//  Created by ivabus on 12.06.2024.
//

import CoreGraphics
import CoreFoundation
import SwiftUI
import MonoLib

#if os(macOS)
typealias PlatformImage = NSImage
#else
typealias PlatformImage = UIImage
#endif

struct Metadata {
	public var title: String
	public var album: String
	public var artist: String
	
	mutating func update() {
		self.title = String(cString: c_get_metadata_title())
		self.album = String(cString: c_get_metadata_album())
		self.artist = String(cString: c_get_metadata_artist())
	}
}

extension Metadata: Equatable {
	static func == (lhs: Self, rhs: Self) -> Bool {
		(lhs.album == rhs.album) && (lhs.artist == rhs.artist) && (lhs.title == rhs.title)
	}
}

struct Cover {
	public var cover: PlatformImage
	
	mutating func update() {
		let cov = c_get_cover_jpeg()
		if cov.length != 0 {
			let data = CFDataCreate(kCFAllocatorDefault, cov.bytes, Int(cov.length))!
#if os(macOS)
			self.cover = PlatformImage(cgImage: CGImage(jpegDataProviderSource: CGDataProvider(data: data)!, decode: nil, shouldInterpolate: false, intent: CGColorRenderingIntent.absoluteColorimetric)!, size: NSSize.init(width: 768, height:768))
#else
			self.cover = PlatformImage(cgImage: CGImage(jpegDataProviderSource: CGDataProvider(data: data)!, decode: nil, shouldInterpolate: false, intent: CGColorRenderingIntent.absoluteColorimetric)!).preparingForDisplay()!
#endif
			// deallocating memory
			c_drop(cov.bytes, UInt(Int(cov.length)))
			print(self.cover.size)
			
		} else {
			self.cover = PlatformImage()
		}
	}
}
