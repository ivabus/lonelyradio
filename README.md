# lonelyradio

> TCP radio for singles

Radio that uses unencrypted TCP socket for broadcasting raw PCM (16/44.1/LE) stream

Decodes audio streams using [symphonia](https://github.com/pdeljanov/Symphonia).

## Build

```shell
cargo build -r
```

## Run

```
lonelyradio [-a <ADDRESS:PORT>] <MUSIC_FOLDER> [-p] [-w]
```

All files (recursively) will be shuffled and played back. Public log will be displayed to stderr, private to stdout.

### Clients

[monoclient](./monoclient) with optional channel separation, hardcoded input (16/44.1/LE).

```shell
monoclient <SERVER>:<PORT> s
```

FFplay (from FFmpeg)

```shell
nc <SERVER> <PORT> | ffplay -f s16le -vn -ac 2 -ar 44100 -nodisp -autoexit -
```

MPV

```shell
nc <SERVER> <PORT> | mpv --audio-channels=stereo --audio-samplerate=44100 --demuxer-rawaudio-format=s16le --demuxer=rawaudio -
```

## License

lonelyradio and monoclient are licensed under the terms of the [MIT license](./LICENSE).
