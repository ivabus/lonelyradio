# lonelyradio Music Streamer

Shuffles through your [XSPF playlists](https://www.xspf.org) or your entire library.

Decodes audio streams using [symphonia](https://github.com/pdeljanov/Symphonia) (supported [decoders](https://github.com/pdeljanov/Symphonia?tab=readme-ov-file#codecs-decoders) and [demuxers](https://github.com/pdeljanov/Symphonia?tab=readme-ov-file#formats-demuxers))

Streams music using [FLAC](https://crates.io/crates/flacenc), [ALAC](https://crates.io/crates/alac-encoder), [Vorbis](https://crates.io/crates/vorbis_rs) or raw PCM on client’s requests.

### Install server

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.7.0 lonelyradio
```

### Run

```
lonelyradio <MUSIC_FOLDER>
```

All files (recursively) will be shuffled and played back. Log will be displayed to stdout.

Look into `--help` for detailed info

#### Run in Docker

```
docker run -d \
   --name lonelyradio \
   --restart=unless-stopped \
   -v /path/to/music:/music \
   -p 5894:5894 \
   ivabuz/lonelyradio:latest
```

#### Playlists

Specify a directory with playlists with `--playlist-dir`. lonelyradio will scan them on startup and play them on clients’ requests.

Only the `<location>` and (playlist's) element would be used and only `file://` is supported.

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
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.7.0 monoclient-s
```

You may need to install some dependencies for Slint.

Desktop integration will be added later.

#### monoclient

[monoclient](./monoclient) is a CLI player for lonelyradio that uses [monolib](./monolib)

##### Install monoclient

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.7.0 monoclient
```

#### Usage

```shell
monoclient <SERVER>:<PORT>
```

Look into `--help` for detailed info on usage.

# Other things

[monoloader](./monoloader) is a tool that allows you to download individual audio tracks from lonelyradio-compatible servers.

[monolib](./monolib) provides a C API compatible with lonelyradio for creating custom clients.

[Protocol documentation] shortly describes the protocol used in lonelyradio. Please refer to monolib and verify custom clients with the reference lonelyradio server.

#### monolib API stability

As lonelyradio has not yet reached its first major release, the API may (and will) break at any point.

## License

lonelyradio, monolib and monoclient, as well as all other crates in this repository, are licensed under the terms of the [MIT license](./LICENSE).
