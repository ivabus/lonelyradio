# lonelyradio

> TCP radio for singles

Radio that uses unencrypted TCP socket for broadcasting tagged audio data.

Decodes audio streams using [symphonia](https://github.com/pdeljanov/Symphonia).

## Install

```shell
cargo install lonelyradio
```

## Build

```shell
cargo build -r
```

## Run

```
lonelyradio <MUSIC_FOLDER> [-a <ADDRESS:PORT>] [-p] [-w] [-m|--max-samplerate M]
```

All files (recursively) will be shuffled and played back. Public log will be displayed to stdout, private to stderr.

`-m|--max-samplerate M` will resample tracks which samplerate exceeds M to M

### Clients

[monoclient](./monoclient) is a recommended CLI client for lonelyradio that uses [monolib](./monolib)

```shell
monoclient <SERVER>:<PORT>
```

### Other clients

SwiftUI client is availible in [platform](./platform) directory.

[monolib](./monolib) provides lonelyradio-compatible C API for creating custom clients.

## License

lonelyradio, monolib and monoclient are licensed under the terms of the [MIT license](./LICENSE).
