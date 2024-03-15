//
//  ContentView.swift
//  monoclient
//
//  Created by ivabus on 03.03.2024.
//

import AVFAudio
import SwiftUI

class MonoLib {
    func run(server: String) async {
        let audioSession = AVAudioSession.sharedInstance()
        do {
            try audioSession.setCategory(
                .playback, mode: .default,
                policy: .longFormAudio)
            try audioSession.setActive(true)

        } catch {
            print("Failed to set the audio session configuration")
        }
        c_start(server)
    }
}

struct ContentView: View {
    let timer = Timer.publish(every: 0.25, on: .main, in: .common).autoconnect()
    @State private var server: String = ""
    @State private var port: String = ""
    @State private var playing: Bool = true
    @State private var running: Bool = false

    @State var now_playing_artist: String = ""
    @State var now_playing_album: String = ""
    @State var now_playing_title: String = ""

    var body: some View {
        VStack {
            Text("Monoclient").font(.largeTitle).fontWidth(.expanded).bold()
            VStack(alignment: .center) {
                HStack {
                    Text("Server").frame(minWidth: 50, idealWidth: 60)
                    TextField(
                        "Required",
                        text: $server
                    )
                    .disableAutocorrection(true)

                }
                .textFieldStyle(.roundedBorder)
                HStack {
                    Text("Port").frame(minWidth: 50, idealWidth: 60)
                    TextField(
                        "Required",
                        text: $port
                    )
                    .disableAutocorrection(true).keyboardType(.numberPad).keyboardShortcut(.escape)
                }
                .textFieldStyle(.roundedBorder)

                Button(action: {
                    if running {
                        playing = !playing
                        c_toggle()
                    }
                    running = true
                    let a = MonoLib()
                    Task.init {
                        await a.run(server: server + ":" + port)
                    }
                }) {
                    Image(
                        systemName: running
                            ? (playing ? "pause.circle.fill" : "play.circle") : "infinity.circle"
                    ).font(.largeTitle)
                }.buttonStyle(
                    .borderedProminent)
                HStack{
                    Button(action: {
                        c_stop()
                        running = false
                        playing = true
                    }) { Image(systemName: "stop").font(.title3) }.buttonStyle(
                        .bordered
                    ).disabled(!running)
                    Button(action: {
                            c_stop()
                        playing = true
                        let a = MonoLib()
                        Task.init {
                            await a.run(server: server + ":" + port)
                        }
                    }) {Image(systemName: "forward").font(.title3)}.buttonStyle(.bordered).disabled(!running)
                }
            }.frame(width: 300)
            VStack(spacing: 10) {
                Text(now_playing_artist).onReceive(timer) { _ in
                    now_playing_artist = String(cString: c_get_metadata_artist()!)
                }
                Text(now_playing_album).onReceive(timer) { _ in
                    now_playing_album = String(cString: c_get_metadata_album()!)
                }
                Text(now_playing_title).onReceive(timer) { _ in
                    now_playing_title = String(cString: c_get_metadata_title()!)
                }.bold()
            }.frame(minHeight: 100)

        }.padding()

    }
}

#Preview {
    ContentView()
}
