# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Voice Transcriber is a native Linux voice-to-text tool for GNOME/Wayland. Press a global hotkey (default: Super+apostrophe) to record audio, which is sent to a Whisper-compatible API, and the transcribed text is copied to the clipboard or injected directly into the focused window.

## Architecture

Four Rust workspace crates + one GNOME Shell extension, communicating over DBus (`org.transcriber.Daemon`):

| Crate | Binary | Role |
|-------|--------|------|
| `common` | (library) | Shared `AppConfig`, DBus constants, config load/save |
| `daemon` | `voice-transcriber-daemon` | Headless systemd service: audio capture (cpal), API calls (reqwest), clipboard (wl-clipboard-rs), history (rusqlite), streaming (tokio-tungstenite) |
| `app` | `voice-transcriber-settings` | GTK4/Libadwaita settings GUI and history viewer |
| `overlay` | `voice-transcriber-overlay` | Wlroots overlay for KDE/Sway/Hyprland (gtk4-layer-shell, ashpd GlobalShortcuts, enigo text injection) |
| `extension/` | (JS) | GNOME Shell extension: hotkey binding, VU meter overlay, DBus client |

The daemon is the core — it owns all state, runs as a systemd user service, and is DBus-activated. UI components are thin clients that call daemon methods and listen for signals.

### DBus Interface (`org.transcriber.Daemon`)

- **Signals:** `StateChanged(state)`, `TranscriptionReady(text)`, `TranscriptionChunk(text)`, `AudioLevel(level)`
- **Methods:** `StartRecording`, `StopRecording`, `Cancel`, `GetHistory(limit)`, `DeleteHistoryEntry(id)`, `ClearHistory`, `RetryTranscription(id, wav_path)`, `ReloadConfig`
- **Property:** `CurrentState` — Idle | Recording | Transcribing | Streaming | Done | Error

### Daemon State Machine

`Idle → Recording → Transcribing → Done` (batch path)
`Idle → Recording → Streaming → Done` (streaming path, requires direct_injection)

VAD (voice activity detection) can auto-trigger stop after ~1.5s silence.

### Data Paths

- Config: `~/.config/voice-transcriber/config.json` (JSON, see `common/src/config.rs` for all fields)
- History DB: `~/.local/share/voice-transcriber/history.db` (SQLite)
- WAV recordings: `~/.local/share/voice-transcriber/recordings/`
- Extension GSettings schema: `extension/schemas/`

## Build & Install

```bash
# Build everything (release)
cargo build --release

# Build just the daemon + settings app (what install.sh builds)
cargo build --release -p voice-transcriber-daemon -p voice-transcriber-settings

# Build the wlroots overlay
cargo build --release -p voice-transcriber-overlay

# Full install (builds, installs binaries/services/extension, enables daemon)
bash install.sh

# Auto-update from git + reinstall
bash update.sh
```

### System Dependencies (Fedora)

```
alsa-lib-devel gtk4-devel libadwaita-devel blueprint-compiler
gtk4-layer-shell-devel   # only for overlay crate
```

Rust 1.92+, GNOME Shell 45–50, libadwaita 1.9+, PipeWire or ALSA.

## Service Management

```bash
# Daemon logs
journalctl --user -u voice-transcriber-daemon.service -f

# Restart daemon after code changes
systemctl --user restart voice-transcriber-daemon.service

# Enable/disable GNOME extension
gnome-extensions enable voice-transcriber@local
gnome-extensions disable voice-transcriber@local

# Extension logs (GNOME Shell)
journalctl /usr/bin/gnome-shell -f
```

## Providers

- **Batch:** Groq (default), Cohere, or custom OpenAI-compatible endpoint
- **Streaming:** Deepgram or AssemblyAI (WebSocket, requires `direct_injection` enabled)

Provider selection is in `AppConfig.provider` / `AppConfig.streaming_provider`. Each has its own API key field. The `active_*()` methods on `AppConfig` resolve the correct URL/key/model for the selected provider.

## Extension Development

The GNOME extension is plain JavaScript/GJS in `extension/`. After editing, reinstall with `bash install.sh` and restart GNOME Shell (Alt+F2 → `r` on X11, or log out/in on Wayland).
