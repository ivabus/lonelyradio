//
//  Player.swift
//  monoclient-x
//
//  Created by ivabus on 13.06.2024.
//

import AVFAudio
import MediaPlayer
import MonoLib
import SwiftUI

enum PlayerState {
	case NotStarted
	case Playing
	case Paused

	mutating func update() {
		self =
			switch c_get_state() {
			case 2: PlayerState.Playing
			case 3: PlayerState.Paused
			default: PlayerState.NotStarted
			}
	}
}

enum EncoderType: UInt8 {
	case PCM16 = 0
	case PCMFloat = 1
	case FLAC = 2
	case Alac = 3
	//WavPack = 4,
	//Opus = 5,
	//Aac = 6,
	case Vorbis = 7
	case Sea = 8
}

enum CoverSize: Int32 {
	case Full = 0
	case High = 768
	case Medium = 512
	case Low = 256
	case Min = 128
	case NoCover = -1
}

struct PlayList: Identifiable, Hashable {
	var id: Int

	var name: String
}

struct Settings {
	var encoder: EncoderType = EncoderType.FLAC
	var cover_size: CoverSize = CoverSize
		.High /*
																						 init(enc: EncoderType, cov: CoverSize) {
																						 encoder = enc
																						 cover_size = cov
																						 }*/
}

#if os(tvOS)
	typealias MyStack = HStack
#else
	typealias MyStack = VStack
#endif

struct Player: View {
	let timer_state = Timer.publish(every: 0.25, on: .main, in: .common).autoconnect()
	let timer_playlists = Timer.publish(every: 5, on: .main, in: .common).autoconnect()
	@State var metadata: Metadata = Metadata(title: "", album: "", artist: "")
	@State var prev_meta: Metadata = Metadata(title: "", album: "", artist: "")
	@State var cover: Cover = Cover(cover: PlatformImage())
	@State var state: PlayerState = PlayerState.NotStarted
	@State var settings: Settings = Settings.init()
	@State var playlists: [PlayList] = [PlayList(id: 0, name: "All tracks")]
	@State var playlist: PlayList = PlayList(id: 0, name: "All tracks")
	@AppStorage("ContentView.server") var server: String = ""

	var body: some View {

		MyStack(alignment: .center) {
			VStack(alignment: .center) {

				#if os(macOS)
					Image(nsImage: cover.cover)
						.resizable()
						.aspectRatio(contentMode: .fit)
						.frame(minWidth: 256, maxWidth: 256, minHeight: 256, maxHeight: 256)
						.frame(width: 256.0, height: 256.0)
						.clipShape(.rect(cornerRadius: 24))
						.shadow(radius: 16)
						.padding(16)
				#else
					Image(uiImage: cover.cover)
						.resizable()
						.aspectRatio(contentMode: .fit)
						.frame(minWidth: 256, maxWidth: 256, minHeight: 256, maxHeight: 256)
						.frame(width: 256.0, height: 256.0)
						.clipShape(.rect(cornerRadius: 24))
						.shadow(radius: 16)
						.padding(16)
				#endif

				VStack(alignment: .center) {
					Text(metadata.title).bold()

					Text(metadata.album)

					Text(metadata.artist)
				}.frame(minHeight: 64).onReceive(timer_state) { _ in
					metadata.update()
					if prev_meta != metadata {
						prev_meta = metadata
						cover.update()
					}
					let image = cover.cover
					let mediaArtwork = MPMediaItemArtwork(boundsSize: image.size) {
						(size: CGSize) -> PlatformImage in
						return image
					}

					#if os(macOS)
						MPNowPlayingInfoCenter.default().playbackState =
							state == PlayerState.Playing ? .playing : .paused
					#endif

					let nowPlayingInfo: [String: Any] = [
						MPMediaItemPropertyArtist: metadata.artist,
						MPMediaItemPropertyAlbumTitle: metadata.album,
						MPMediaItemPropertyTitle: metadata.title,
						MPMediaItemPropertyArtwork: mediaArtwork,
						MPNowPlayingInfoPropertyIsLiveStream: true,
						MPMediaItemPropertyPlaybackDuration: c_get_metadata_length(),

					]
					MPNowPlayingInfoCenter.default().nowPlayingInfo = nowPlayingInfo
				}

				TextField(
					"Server",
					text: $server,
					onCommit: {
						#if os(macOS)
							DispatchQueue.main.async {
								NSApp.keyWindow?.makeFirstResponder(nil)
							}
						#endif
					}
				)
				.disableAutocorrection(true)
				.frame(width: 256)
				.textFieldStyle(.roundedBorder)
				.padding(16)
				.multilineTextAlignment(.center)

				HStack(spacing: 8) {
					Button(action: stop) {
						Image(systemName: "stop.fill").padding(4).frame(width: 32, height: 24)
					}
					.disabled(state == PlayerState.NotStarted)
					.buttonStyle(.bordered)
					.font(.system(size: 20))
					.buttonBorderShape(.capsule)

					Button(action: play) {
						Image(
							systemName: state == PlayerState.NotStarted
								? "infinity.circle"
								: (state == PlayerState.Playing)
									? "pause.circle.fill" : "play.circle"
						)
						.font(.system(size: 30))
						.padding(4)
					}
					.buttonStyle(.borderedProminent)
					.buttonBorderShape(.capsule)

					Button(action: next) {
						Image(systemName: "forward.end.fill").padding(4).frame(
							width: 32, height: 24)
					}.disabled(state == PlayerState.NotStarted)
						.buttonStyle(.bordered)
						.font(.system(size: 20))
						.buttonBorderShape(.capsule)
				}.onReceive(timer_state) { _ in
					state.update()
				}
				Menu {
					Picker("Playlist", selection: $playlist) {
						ForEach($playlists) { pl in
							Text(pl.wrappedValue.name).tag(pl.wrappedValue)
						}

					}.pickerStyle(.menu)

					Picker("Encoder", selection: $settings.encoder) {
						Text("PCM (s16)")
							.tag(EncoderType.PCM16)
						Text("PCM (f32)")
							.tag(EncoderType.PCMFloat)
						Text("FLAC (s24)")
							.tag(EncoderType.FLAC)
						Text("ALAC (s16)")
							.tag(EncoderType.Alac)
						Text("Vorbis (lossy)")
							.tag(EncoderType.Vorbis)
						Text("Sea (lossy)")
							.tag(EncoderType.Sea)
					}.pickerStyle(.menu)

					Picker("Cover size", selection: $settings.cover_size) {
						Text("Original")
							.tag(CoverSize.Full)
						Text("High (768)")
							.tag(CoverSize.High)
						Text("Medium (512)")
							.tag(CoverSize.Medium)
						Text("Low (256)")
							.tag(CoverSize.Low)
						Text("Min (128)")
							.tag(CoverSize.Min)
						Text("No cover")
							.tag(CoverSize.NoCover)
					}.pickerStyle(.menu)
				} label: {
					Label("Settings", systemImage: "gearshape")
						.padding(16)
				}.frame(maxWidth: 128)
			}

			.padding(32)
			.onReceive(timer_playlists) { _ in
				var id = -1
				playlists =
					(["All tracks"]
					+ String(cString: c_list_playlists(server)).components(separatedBy: "\n")).map({
						elem in
						if elem.isEmpty {
							return PlayList(id: -1, name: elem)
						}
						id += 1
						return PlayList(id: id, name: elem)
					}).filter({ elem in elem.id != -1 })
			}
			.onAppear {
				#if os(iOS)
					UIApplication.shared.beginReceivingRemoteControlEvents()
				#endif
				MPRemoteCommandCenter.shared().previousTrackCommand.isEnabled = false
				MPRemoteCommandCenter.shared().nextTrackCommand.isEnabled = true
				MPRemoteCommandCenter.shared().skipForwardCommand.isEnabled = false
				MPRemoteCommandCenter.shared().skipBackwardCommand.isEnabled = false
				MPRemoteCommandCenter.shared().pauseCommand.addTarget(handler: { _ in
					if state != PlayerState.Paused {
						play()
					}
					return MPRemoteCommandHandlerStatus.success
				})
				MPRemoteCommandCenter.shared().playCommand.addTarget(handler: { _ in
					if state != PlayerState.Playing {
						play()
					}
					return MPRemoteCommandHandlerStatus.success
				})

				MPRemoteCommandCenter.shared().togglePlayPauseCommand.addTarget(handler: { _ in
					play()
					return MPRemoteCommandHandlerStatus.success
				})

				MPRemoteCommandCenter.shared().nextTrackCommand.addTarget(handler: { _ in
					next()
					return MPRemoteCommandHandlerStatus.success
				})

			}

		}
		.animation(.spring, value: UUID())
	}

	func play() {
		switch state {
		case PlayerState.NotStarted:
			do {
				#if os(macOS)
				#else
					let audioSession = AVAudioSession.sharedInstance()
					do {
						try audioSession.setCategory(
							.playback, mode: .default)
						try audioSession.setActive(true)

					} catch {
						print("Failed to set the audio session configuration")
					}
				#endif
				Thread.detachNewThread {
					c_start(
						server,
						CSettings(
							encoder: settings.encoder.rawValue, cover: settings.cover_size.rawValue),
						playlist.name == "All tracks" ? "" : playlist.name)
				}
			}
		default:
			do {
				c_toggle()
				state.update()
			}
		}

	}
	func stop() {
		c_stop()
		state.update()
		cover = Cover(cover: PlatformImage())
	}
	func next() {
		c_stop()
		state.update()
		play()
	}
}
