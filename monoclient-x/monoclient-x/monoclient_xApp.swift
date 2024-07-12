//
//  monoclient_xApp.swift
//  monoclient-x
//
//  Created by ivabus on 12.06.2024.
//

import SwiftUI

@main
struct monoclient_xApp: App {
	
	var body: some Scene {
#if os(macOS)
		WindowGroup {
			ContentView().onAppear {
				NSWindow.allowsAutomaticWindowTabbing = false
			}
			.containerBackground(.ultraThinMaterial, for: .window)
			.windowFullScreenBehavior(.disabled)
			.windowResizeBehavior(.disabled)
		}.defaultSize(width: 256, height: 512)
			.windowStyle(.hiddenTitleBar)
			.commands {
				CommandGroup(replacing: CommandGroupPlacement.newItem) {
				}
			}
#else
		WindowGroup {
			ContentView()
		}
		.defaultSize(width: 256, height: 512)
#endif
	}
}
