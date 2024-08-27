# lonelyradio protocol (0.7)

## Introduction

The lonelyradio protocol is designed to be minimal yet functional for music streaming purposes.

The lonelyradio protocol operates at the application layer, establishing communication between the server and client. In its reference implementation, it runs atop the TCP protocol, but it could also be implemented on top of any other transport protocol, such as UDP, WebSocket, and so on.

The lonelyradio protocol uses [MessagePack](https://msgpack.org) to encode messages. Structures used in communication are defined in the `lonelyradio_types` crate.

## Establishing connection

1. The client sends a «hello» packet («lonelyra», 8 bytes)
    1. The server checks the hello packet
2. The server sends «ServerCapabilities» which informs the client about supported audio encoders (raw pcm s16le must be supported by all server implementations)
3. Then the client picks one of the requests:
    1. Play (p) (see example 1.1)
    2. ListPlaylist (lpl) (see example 1.2)
    3. PlayPlayList (ppl) (see example 1.3)
4. The server responds with one of RequestResult
    1. Ok -> The server begins sending PlayMessage’s
        1. TrackMetadata indicates the start of the new track
        2. FragmentMetadata indicates the start of a new fragment and defines the number of bytes in it
            1. FragmentMetadata is always followed by a fragment
    2. Playlist is only returned on ListPlaylist and shows available playlists
    3. Error indicates an error

To get «next track» just reestablish the connection.

## Examples

Examples show JSON representation of MessagePack

### 1.1

```json
{
  "p": { // e and co definicions could be found in	lonelyradio_types crate
    "e": "Pcm16",
    "co": -1
  }
}
```

### 1.2

Just string encoded to MessagePack

```json
"lpl"
```

### 1.3

```json
{
  "ppl": [
    "someplaylist",
    {
      "e": "Pcm16",
      "co": -1
    }
  ]
}
```
