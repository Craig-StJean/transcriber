# Voice Transcriber

A native Linux voice-to-text tool for Wayland. Press a global hotkey to record, and your transcribed text is automatically copied to the clipboard.

Supported sessions:

- **GNOME on Fedora** (and similar) — via the bundled GNOME Shell extension. Install with `bash install.sh`.
- **Hyprland / wlroots / KDE Plasma 6** — via the `voice-transcriber-overlay` binary (`gtk4-layer-shell` + DBus hotkeys). On NixOS, install via the flake (see [docs/hyprland.md](docs/hyprland.md)).

## How it works

1. Press **Super+'** (Super + apostrophe) to start recording
2. An overlay appears showing a live VU meter
3. Press **Super+'** again to stop
4. Audio is sent to a Whisper-compatible API (default: Groq)
5. Transcribed text is copied to your clipboard automatically
6. View history and configure settings via the Settings app

## Architecture

The project has four components communicating over DBus:

| Component | Language | Role |
|-----------|----------|------|
| **Daemon** | Rust | Headless service — audio capture, API calls, clipboard injection, history DB |
| **Settings App** | Rust + GTK4/Libadwaita | GUI for configuration and transcription history |
| **Overlay** | Rust + GTK4 (`gtk4-layer-shell`) | On-screen VU meter for wlroots/KDE compositors (Hyprland, Sway, Plasma) |
| **GNOME Extension** | JavaScript (GJS) | Hotkey binding and on-screen overlay for GNOME Shell |

Pick one of **Overlay** or **GNOME Extension** depending on your session. The daemon runs as a systemd user service and is activated on first DBus contact.

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

- A Wayland session: GNOME (Fedora/Ubuntu), Hyprland, Sway, or KDE Plasma 6
- Rust 1.92+ (only when building from source)
- PipeWire or ALSA audio
- An API key for a Whisper-compatible service (e.g. [Groq](https://console.groq.com))

## Quick Start

### Fedora / GNOME

See [INSTALL.md](INSTALL.md) for full installation instructions.

```bash
bash install.sh
voice-transcriber-settings   # configure your API key
gnome-extensions enable voice-transcriber@local
# press Super+' to record
```

### NixOS / Hyprland

Full guide: [docs/hyprland.md](docs/hyprland.md). Short version:

```nix
# home.nix
{ inputs, ... }: {
  imports = [ inputs.voice-transcriber.homeManagerModules.default ];
  programs.voice-transcriber = {
    enable = true;
    waylandDisplay = "wayland-1";
  };
}
```

```conf
# ~/.config/hypr/hyprland.conf
bind = SUPER, apostrophe, exec, dbus-send --session --type=method_call \
    --dest=org.transcriber.Daemon /org/transcriber/Daemon \
    org.transcriber.Daemon.StartRecording
```

`nix develop` gives a build shell; `nix build .#voice-transcriber` builds the binaries.

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
