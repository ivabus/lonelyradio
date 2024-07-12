//
//  ContentView.swift
//  monoclient-x
//
//  Created by ivabus on 12.06.2024.
//

import SwiftUI
import SwiftData
import AppIntents

struct ContentView: View {
	var body: some View {
		Player()
	}
}

struct PlayIntent: AudioPlaybackIntent {
	static var title: LocalizedStringResource = "Start lonelyradio client"
	static var description = IntentDescription("Plays from setted up server")
	
	static var openAppWhenRun: Bool = false
	static var isDiscoverable: Bool = true
	
	@MainActor
	func perform() async throws -> some IntentResult {
		Player().play()
		return .result()
	}
}

struct StopIntent: AudioPlaybackIntent {
	static var title: LocalizedStringResource = "Stop lonelyradio client"
	static var description = IntentDescription("Stops monoclient")
	
	static var openAppWhenRun: Bool = false
	static var isDiscoverable: Bool = true
	
	@MainActor
	func perform() async throws -> some IntentResult {
		Player().stop()
		return .result()
	}
}

struct LibraryAppShortcuts: AppShortcutsProvider {
	static var appShortcuts: [AppShortcut] {
		AppShortcut(
			intent: PlayIntent(),
			phrases: [
				"Start playback \(.applicationName)",
			],
			shortTitle: "Start monoclient",
			systemImageName: "infinity.circle"
		)
		AppShortcut(
			intent: StopIntent(),
			phrases: [
				"Stop playback in \(.applicationName)"
			],
			shortTitle: "Stop monoclient",
			systemImageName: "stop.fill"
		)
	}
	
	
}

#Preview {
	ContentView()
}
