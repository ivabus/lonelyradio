# lonelyradio

Broadcast audio over the internet.

Decodes audio streams using [symphonia](https://github.com/pdeljanov/Symphonia).

Optionally transcodes audio into and from FLAC using [flacenc-rs](https://github.com/yotarok/flacenc-rs/) and [claxon](https://github.com/ruuda/claxon).

## Installation

### Install music server

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.5.0 lonelyradio
```

### Install CLI client

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.5.0 monoclient
```

### Install GUI (Slint) client

```shell
cargo install --git https://github.com/ivabus/lonelyradio --tag 0.5.0 monoclient-s
```

## Run

```
lonelyradio [-a <ADDRESS:PORT>] [-p|--public-log] [-w|--war] [-m|--max-samplerate M] [--xor-key-file FILE] [--no-resampling] [-f|--flac] <MUSIC_FOLDER>
```

All files (recursively) will be shuffled and played back. Public log will be displayed to stdout, private to stderr.

`-m|--max-samplerate M` will resample tracks which samplerate exceeds M to M

`--xor-key-file FILE` will XOR all outgoing bytes looping through FILE

`-f|--flac` will enable (experimental) FLAC compression

### Clients

[monoclient](./monoclient) is a recommended CLI player for lonelyradio that uses [monolib](./monolib)

```shell
monoclient <SERVER>:<PORT>
```

[monoclient-s](./monoclient-s) is a experimental GUI player for lonelyradio built with [Slint](https://slint.dev)

```shell
monoclient-s
```

Desktop integration will be added later.

### Other clients

SwiftUI client is availible in [platform](./platform) directory.

[monoloader](./monoloader) is a tool, that allows you to download individual audio tracks from lonelyradio-compatible servers.

[monolib](./monolib) provides a C API compatible with lonelyradio for creating custom clients.

#### monolib API stability

As lonelyradio has not yet reached its first major release, the API may (and will) break at any point.

### Microphone server

Experimental server (lonelyradio-compatible) for streaming audio from your microphone is available in the [microserve](./microserve) crate.

## License

lonelyradio, monolib and monoclient, as well as all other crates in this repository, are licensed under the terms of the [MIT license](./LICENSE).
