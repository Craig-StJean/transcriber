# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Transcriber is a native Linux voice-to-text tool for GNOME/Wayland. Press a global hotkey (default: Super+apostrophe) to record audio, which is sent to a Whisper-compatible API, and the transcribed text is copied to the clipboard or injected directly into the focused window.

## Architecture

Four Rust workspace crates + one GNOME Shell extension, communicating over DBus (`org.transcriber.Daemon`):

| Crate | Binary | Role |
|-------|--------|------|
| `common` | (library) | Shared `AppConfig`, DBus constants, config load/save |
| `daemon` | `transcriber-daemon` | Headless systemd service: audio capture (cpal), API calls (reqwest), clipboard (wl-clipboard-rs), history (rusqlite), streaming (tokio-tungstenite) |
| `app` | `transcriber-settings` | GTK4/Libadwaita settings GUI and history viewer |
| `overlay` | `transcriber-overlay` | Wlroots overlay for KDE/Sway/Hyprland (gtk4-layer-shell, ashpd GlobalShortcuts, enigo text injection) |
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

- Config: `~/.config/transcriber/config.json` (JSON, see `common/src/config.rs` for all fields)
- History DB: `~/.local/share/transcriber/history.db` (SQLite)
- WAV recordings: `~/.local/share/transcriber/recordings/`
- Extension GSettings schema: `extension/schemas/`

## Build & Install

```bash
# Build everything (release)
cargo build --release

# Build just the daemon + settings app (what install.sh builds)
cargo build --release -p transcriber-daemon -p transcriber-settings

# Build the wlroots overlay
cargo build --release -p transcriber-overlay

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

Rust 1.91+, GNOME Shell 45–50, libadwaita 1.9+, PipeWire or ALSA.

## Service Management

```bash
# Daemon logs
journalctl --user -u transcriber-daemon.service -f

# Restart daemon after code changes
systemctl --user restart transcriber-daemon.service

# Enable/disable GNOME extension
gnome-extensions enable transcriber@local
gnome-extensions disable transcriber@local

# Extension logs (GNOME Shell)
journalctl /usr/bin/gnome-shell -f
```

## Providers

- **Batch:** Groq (default), Cohere, or custom OpenAI-compatible endpoint
- **Streaming:** Deepgram or AssemblyAI (WebSocket, requires `direct_injection` enabled)

Provider selection is in `AppConfig.provider` / `AppConfig.streaming_provider`. Each has its own API key field. The `active_*()` methods on `AppConfig` resolve the correct URL/key/model for the selected provider.

### Vocabulary hints

`AppConfig.vocabulary` is a list of domain terms sent as Whisper's `prompt` parameter to bias spelling. `active_prompt()` renders it, and owns three rules worth knowing before touching it:

- **Cohere is excluded** — its endpoint documents no `prompt` field, and an unrecognised multipart field there would 400 every request, turning a cosmetic feature into an outage. Custom endpoints *are* included (OpenAI-compatible by definition).
- **Truncation is ours, not the API's.** Whisper silently drops everything past 224 tokens, so `active_prompt()` cuts at a term boundary first. `estimate_prompt_tokens()` intentionally over-counts (3 chars/token) because this field holds proper nouns, which BPE splits far worse than prose.
- **The prompt is a bare comma-separated list**, not a sentence. Whisper conditions on it as preceding transcript text, so instructions written into it are not followed — they leak into the output as transcribed words.

On NixOS, `home/transcriber.nix` in the system-config repo merges its declared fields *onto* the existing `config.json` rather than rebuilding it, so GUI-edited vocabulary survives a switch. Adding a field there that the GUI also owns will clobber it.

## Extension Development

The GNOME extension is plain JavaScript/GJS in `extension/`. After editing, reinstall with `bash install.sh` and restart GNOME Shell (Alt+F2 → `r` on X11, or log out/in on Wayland).
