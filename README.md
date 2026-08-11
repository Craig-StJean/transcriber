# Transcriber

A native Linux voice-to-text tool for Wayland. Press a global hotkey to record, and your transcribed text is automatically copied to the clipboard.

Supported sessions:

- **GNOME on Fedora** (and similar) — via the bundled GNOME Shell extension. Install with `bash install.sh`.
- **Hyprland / wlroots / KDE Plasma 6** — via the `transcriber-overlay` binary (`gtk4-layer-shell` + DBus hotkeys). On NixOS, install via the flake (see [docs/hyprland.md](docs/hyprland.md)).

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

Settings are stored in `~/.config/transcriber/config.json` and managed via the Settings app (`transcriber-settings`).

| Setting | Default | Description |
|---------|---------|-------------|
| `api_url` | `https://api.groq.com/openai/v1/audio/transcriptions` | OpenAI-compatible transcription endpoint |
| `api_key` | *(required)* | API authentication token |
| `model` | `whisper-large-v3-turbo` | Whisper model name |
| `language` | `en` | BCP-47 language hint (optional) |
| `sample_rate` | `16000` | Microphone sample rate in Hz |
| `vocabulary` | `["AccuDose", "Safehous"]` | Spelling hints — see below |
| `vocabulary_enabled` | `true` | Whether to send them |

Any OpenAI-compatible Whisper endpoint works (Groq, OpenAI, local Whisper server, etc.).

### Vocabulary hints

Whisper mangles words it hasn't seen — product names, invented spellings, jargon.
The **Vocabulary** group on the Settings page takes one term per line and sends
them with every request as Whisper's `prompt` parameter, which biases the decoder
toward those spellings.

Notes:

- It is a *hint*, not a constraint. Whisper can still produce something else, and
  a strong enough hint can make it produce a listed term from unclear audio.
- The cap is **224 tokens**, enforced upstream by silent truncation. The GUI shows
  a running estimate and the daemon drops whole terms off the end of the list
  rather than letting the API cut mid-word. The estimate deliberately over-counts,
  since proper nouns tokenize worse than prose.
- **Cohere is excluded.** Its endpoint is only OpenAI-shaped and documents no
  `prompt` field, so the hint is withheld there rather than risk a 400 on every
  request. The Settings group greys out when Cohere is selected. Custom endpoints
  do get it.
- Streaming providers (Deepgram, AssemblyAI) have their own keyword-boost
  mechanisms and are not wired to this list.

Transcription history is stored in `~/.local/share/transcriber/history.db` (SQLite).

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
transcriber-settings   # configure your API key
gnome-extensions enable transcriber@local
# press Super+' to record
```

### NixOS / Hyprland

Full guide: [docs/hyprland.md](docs/hyprland.md). Short version:

```nix
# home.nix
{ inputs, ... }: {
  imports = [ inputs.transcriber.homeManagerModules.default ];
  programs.transcriber = {
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

`nix develop` gives a build shell; `nix build .#transcriber` builds the binaries.

#### Adding it as a flake input

```nix
inputs.transcriber = {
  url = "github:Craig-StJean/transcriber/v0.2.0";   # pin to a tag
  inputs.nixpkgs.follows = "nixpkgs";               # share your nixpkgs
};
```

**Pin to a tag, not to `main`.** A bare `github:Craig-StJean/transcriber` tracks
the default branch, so `nix flake update` pulls whatever happens to be on it.
Pinning to a tag means the machine only moves when you change that string. Track
`main` only on a machine you're happy to break.

**Set `follows`.** This flake pins a NixOS release channel for standalone builds;
`follows` points it at yours instead, so you get one nixpkgs closure rather than
two. Verified against `nixos-26.05`. (Before 2026-08-10 this was impossible —
the flake needed `staging-next` for libadwaita 1.9, which no release channel had
yet. That is no longer the case, and the pin has been removed.)

#### Binary cache

CI pushes every build to a Cachix cache, so consumers download the binaries
instead of compiling Rust and GTK4:

```nix
nix.settings = {
  substituters       = [ "https://craig-transcriber.cachix.org" ];
  trusted-public-keys = [ "craig-transcriber.cachix.org-1:<key>" ];
};
```

Both lines are additive — put them alongside whatever caches you already use;
don't replace `cache.nixos.org`. The `<key>` is shown when the cache is created.

### Releasing

```sh
git tag -a v0.2.0 -m "vocabulary hints"
git push origin v0.2.0
```

That fires two workflows: [`nix.yml`](.github/workflows/nix.yml) builds the flake
and populates the binary cache, and [`release.yml`](.github/workflows/release.yml)
builds the Fedora tarball for the non-Nix install path and attaches it to a
GitHub Release. Consumers on Nix bump their pinned tag to pick it up.

## Troubleshooting

**Daemon not running:**
```bash
systemctl --user status transcriber-daemon.service
journalctl --user -u transcriber-daemon.service -f
```

**Extension not loading:**
```bash
gnome-extensions list
gnome-extensions enable transcriber@local
```

**Clipboard not populated after transcription:**
The daemon requires `WAYLAND_DISPLAY` to be set. This is handled automatically by the systemd service. If running manually, ensure `WAYLAND_DISPLAY=wayland-0` is set in your environment.

**Audio not captured:**
Check that your user has access to the audio device:
```bash
arecord -l        # list capture devices
```
