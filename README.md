# lonelyradio

Broadcast lossless audio over the internet.

Decodes audio streams using [symphonia](https://github.com/pdeljanov/Symphonia).

Optionally transcodes audio into and from FLAC using [flacenc-rs](https://github.com/yotarok/flacenc-rs/) and [claxon](https://github.com/ruuda/claxon).

## Install server

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.6.0 lonelyradio
```

## Run

```
lonelyradio <MUSIC_FOLDER>
```

All files (recursively) will be shuffled and played back. Public log will be displayed to stdout, private to stderr.

Look into `--help` for detailed info

### Clients

#### monoclient-x

[monoclient-x](./monoclient-x) is a SwiftUI player for lonelyradio for iOS/iPadOS/macOS

##### Build

1. Build monolib with [xcframework](https://github.com/Binlogo/cargo-xcframework)
2. Build monoclient-x using Xcode or `xcodebuild`

#### monoclient-s

[monoclient-s](./monoclient-s) is a GUI player for lonelyradio built with [Slint](https://slint.dev)


##### Install

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.6.0 monoclient-s
```

You may need to install some dependencies for Slint.

Desktop integration will be added later.

##### Build

```
cargo build -p monoclient-s
```

You may need to install some dependencies for Slint.

#### monoclient

[monoclient](./monoclient) is a CLI player for lonelyradio that uses [monolib](./monolib)

```shell
monoclient <SERVER>:<PORT>
```

##### Install monoclient

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.6.0 monoclient
```

# Other things

[monoloader](./monoloader) is a tool, that allows you to download individual audio tracks from lonelyradio-compatible servers.

[monolib](./monolib) provides a C API compatible with lonelyradio for creating custom clients.

The full protocol specification will be available later. If you would like to learn more about it now, please refer to the monolib.

#### monolib API stability

As lonelyradio has not yet reached its first major release, the API may (and will) break at any point.

### Microphone server

Experimental (and uncompatible with versions 0.6+) server (lonelyradio-compatible) for streaming audio from your microphone is available in the [microserve](./microserve) crate.

## License

lonelyradio, monolib and monoclient, as well as all other crates in this repository, are licensed under the terms of the [MIT license](./LICENSE).
