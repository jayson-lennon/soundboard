# Soundboard Application Context

## Overview

Soundboard is a simple audio playback server with varlink IPC. It runs as a user-level systemd service, activated when a duckyPad device is connected.

## Architecture

- **Single binary** with two modes: server and client
- **Server mode**: Listens on varlink socket, plays audio files on request
- **Client mode**: Sends play commands to the server via varlink

## Key Components

### CLI (src/main.rs)
- Uses clap with subcommands: `server` and `play`
- Server mode: varlink service with idle timeout
- Client mode: connects to varlink socket, sends play request

### Varlink Interface (src/lib.rs)
- Interface: `io.soundboard`
- Method: `Play(path: string) -> PlayResponse`
- Uses zlink crate for varlink implementation

### Audio Player (src/lib.rs)
- Uses rodio for audio playback
- Supports PipeWire via PulseAudio compatibility
- Default output device, configurable via config file

## Configuration

Config file location: `~/.config/soundboard/config.toml`

```toml
output_device = ""  # Optional: specific audio device name
```

## Systemd Integration

- **udev rule**: Triggers socket activation on duckyPad connect
- **Socket unit**: Creates `/run/user/$UID/soundboard.varlink`
- **Service unit**: Activated on-demand when client connects

### Device Detection
- duckyPad VID: `0x0483`
- duckyPad PID: `0xD11C`

## Error Handling

All errors use `wherror::Error` with `error_stack::Report`.

### Error Types
- `ConfigError`: Configuration loading failures
- `AudioError`: Audio playback failures
- `SoundboardError`: Varlink service errors (FileNotFound, PlaybackFailed)

## Usage

```bash
# Play a sound (client mode)
soundboard play /path/to/sound.mp3

# Run server manually (normally started by systemd)
soundboard server
```

## Idle Timeout

Server exits after 5 minutes of inactivity to conserve resources. The socket remains and will re-activate the service when needed.
