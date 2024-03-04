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
        start(server)
    }
}

struct ContentView: View {
    @State private var server: String = ""
    @State private var port: String = ""
    @State private var playing: Bool = true
    @State private var running: Bool = false

    var body: some View {
        VStack {
            Text("Monoclient").font(.largeTitle).fontWidth(.expanded).bold()  //.padding(.top, 25)
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
                        toggle()
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
                Button(action: {
                    reset()
                    running = false
                    playing = true
                }) { Image(systemName: "stop").font(.title3) }.buttonStyle(
                    .bordered
                ).disabled(!running)
            }.frame(width: 300)

        }.padding()

    }
}

#Preview {
    ContentView()
}
