# Voice Transcriber

A native Linux voice-to-text tool for GNOME on Wayland. Press a global hotkey to record, and your transcribed text is automatically copied to the clipboard.

## How it works

1. Press **Super+`** to start recording
2. An overlay appears showing a live VU meter
3. Press **Super+`** again to stop
4. Audio is sent to a Whisper-compatible API (default: Groq)
5. Transcribed text is copied to your clipboard automatically
6. View history and configure settings via the Settings app

## Architecture

The project has three components communicating over DBus:

| Component | Language | Role |
|-----------|----------|------|
| **Daemon** | Rust | Headless service — audio capture, API calls, clipboard injection, history DB |
| **Settings App** | Rust + GTK4/Libadwaita | GUI for configuration and transcription history |
| **GNOME Extension** | JavaScript (GJS) | Hotkey binding and on-screen recording overlay |

The daemon runs as a systemd user service and is activated on first DBus contact. The GNOME extension only handles display; the daemon handles everything else.

## Configuration

Settings are stored in `~/.config/voice-transcriber/config.json` and managed via the Settings app (`voice-transcriber-settings`).

| Setting | Default | Description |
|---------|---------|-------------|
| `api_url` | `https://api.groq.com/openai/v1/audio/transcriptions` | OpenAI-compatible transcription endpoint |
| `api_key` | *(required)* | API authentication token |
| `model` | `whisper-large-v3-turbo` | Whisper model name |
| `language` | `en` | BCP-47 language hint (optional) |
| `sample_rate` | `16000` | Microphone sample rate in Hz |

Any OpenAI-compatible Whisper endpoint works (Groq, OpenAI, local Whisper server, etc.).

Transcription history is stored in `~/.local/share/voice-transcriber/history.db` (SQLite).

## Requirements

- Fedora / GNOME on Wayland
- GNOME Shell 45–47
- Rust 1.92+
- PipeWire or ALSA audio
- An API key for a Whisper-compatible service (e.g. [Groq](https://console.groq.com))

## Quick Start

See [INSTALL.md](INSTALL.md) for full installation instructions.

```bash
bash install.sh
voice-transcriber-settings   # configure your API key
gnome-extensions enable voice-transcriber@local
# press Super+` to record
```

## Troubleshooting

**Daemon not running:**
```bash
systemctl --user status voice-transcriber-daemon.service
journalctl --user -u voice-transcriber-daemon.service -f
```

**Extension not loading:**
```bash
gnome-extensions list
gnome-extensions enable voice-transcriber@local
```

**Clipboard not populated after transcription:**
The daemon requires `WAYLAND_DISPLAY` to be set. This is handled automatically by the systemd service. If running manually, ensure `WAYLAND_DISPLAY=wayland-0` is set in your environment.

**Audio not captured:**
Check that your user has access to the audio device:
```bash
arecord -l        # list capture devices
```
