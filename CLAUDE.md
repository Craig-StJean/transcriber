# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Transcriber is a native Linux voice-to-text tool for GNOME/Wayland. Press a global hotkey (default: Super+apostrophe) to record audio, which is sent to a Whisper-compatible API, and the transcribed text is copied to the clipboard or injected directly into the focused window.

## Architecture

Four Rust workspace crates + one GNOME Shell extension, communicating over DBus (`org.transcriber.Daemon`):

| Crate | Binary | Role |
|-------|--------|------|
| `common` | (library) | Shared `AppConfig`, DBus constants, config load/save |
| `daemon` | `transcriber-daemon` | Headless systemd service: audio capture (cpal), API calls (reqwest), history (rusqlite), streaming (tokio-tungstenite). No clipboard — clients (extension/overlay) deliver the text |
| `app` | `transcriber-settings` | GTK4/Libadwaita settings GUI and history viewer |
| `overlay` | `transcriber-overlay` | Wlroots overlay for KDE/Sway/Hyprland (gtk4-layer-shell, ashpd GlobalShortcuts, enigo text injection) |
| `extension/` | (JS) | GNOME Shell extension: hotkey binding, VU meter overlay, DBus client |

The daemon is the core — it owns all state, runs as a systemd user service, and is DBus-activated. UI components are thin clients that call daemon methods and listen for signals.

### DBus Interface (`org.transcriber.Daemon`)

- **Signals:** `StateChanged(state)`, `ErrorOccurred(message)` (sent just before `StateChanged("Error")`), `SessionDiscarded(reason)` (`"cancelled"` | `"no-speech"`, sent just before `StateChanged("Idle")` when a session ends without text), `TranscriptionReady(text)`, `TranscriptionChunk(text)` (cumulative stream text so far), `AudioLevel(level)`
- **Methods:** `StartRecording`, `StopRecording`, `Cancel`, `GetHistory(limit)`, `DeleteHistoryEntry(id)`, `ClearHistory`, `RetryTranscription(id, wav_path)`, `RepolishEntry(id) → text`, `TestConnections() → JSON`, `ReloadConfig`
  - `TestConnections` returns `[{"name","ok","detail"}]` for each provider in use (logic in `daemon/src/connections.rs`); the settings Status page shows it.
  - `RepolishEntry` re-runs post-processing on a history entry from its original text; works whenever post-processing is configured, even if it's switched off.
  - `RetryTranscription` ignores `wav_path` (kept for compatibility) and looks the WAV up by id; it runs post-processing like a normal recording.
  - `DeleteHistoryEntry` / `ClearHistory` also delete the WAVs. The settings app goes through these rather than writing SQLite itself.
- **Property:** `CurrentState` — Idle | Recording | Transcribing | PostProcessing | Streaming | Done | Error (emits `PropertiesChanged`)

### Daemon State Machine

`Idle → Recording → Transcribing → [PostProcessing →] Done` (batch path)
`Idle → Recording → Streaming → Done` (streaming path, requires direct_injection)

- Every `StartRecording`/`Cancel` bumps a session *generation*; background tasks only emit signals, write history or change state while their generation is current. That is what makes Cancel silent and stops a stale pipeline clobbering a newer recording. Keep new async work gated on it.
- `StopRecording`, VAD (voice activity detection) auto-stop after ~1.5 s silence, and the `max_recording_secs` cap all go through one `stop()` that compare-and-sets out of Recording, so only one of them wins.
- Empty recordings (< 0.3 s, or no speech seen with VAD on) go straight to Idle with no API call and no history row.

### Data Paths

- Config: `~/.config/transcriber/config.json` (JSON, see `common/src/config.rs` for all fields)
- History DB: `~/.local/share/transcriber/history.db` (SQLite)
- WAV recordings: `~/.local/share/transcriber/recordings/`
- These follow `$XDG_CONFIG_HOME` / `$XDG_DATA_HOME`. The sandboxed daemon unit uses `%E` for config, but systemd has no data-dir specifier, so a custom `XDG_DATA_HOME` needs a drop-in adding it to `ReadWritePaths` (the home-manager module handles this itself via `xdg.dataHome`).
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
alsa-lib-devel gtk4-devel libadwaita-devel
gtk4-layer-shell-devel libxkbcommon-devel   # only for overlay crate
```

Rust 1.92+ (pinned in `rust-toolchain.toml`), GNOME Shell 47–50, libadwaita 1.9+, PipeWire or ALSA.

On NixOS, plain `cargo` can't find ALSA; run it via `nix develop --command cargo …`.

The code hand-aligns `=` and match arms into columns — don't run `cargo fmt` over it. CI runs clippy with `-D warnings` and the workspace tests.

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

- **Batch:** Groq (default) or a custom OpenAI-compatible endpoint
- **Streaming:** Deepgram (WebSocket, requires `direct_injection` enabled)
- **Post-processing:** Groq, Gemini, or custom (OpenAI-compatible chat completions)

Cohere and AssemblyAI were tried and removed; `config::load()` maps a stale `provider: "cohere"` back to Groq, and serde ignores their old keys. The `active_*()` methods on `AppConfig` resolve URL/key/model for the selected provider.

### Vocabulary hints

`AppConfig.vocabulary` is a list of domain terms sent as Whisper's `prompt` parameter to bias spelling. `active_prompt()` renders it, and owns these rules worth knowing before touching it:

- **Truncation is ours, not the API's.** Whisper silently drops everything past 224 tokens, so `active_prompt()` cuts at a term boundary first. `estimate_prompt_tokens()` intentionally over-counts (3 chars/token) because this field holds proper nouns, which BPE splits far worse than prose.
- **The prompt is a bare comma-separated list**, not a sentence. Whisper conditions on it as preceding transcript text, so instructions written into it are not followed — they leak into the output as transcribed words.
- **Silence echoes the prompt.** On audio with no speech Whisper returns the vocabulary list as the transcript. `is_prompt_echo()` catches that and the daemon treats it as an empty (no-speech) result.

On NixOS, the home-manager module (`nix/hm-module.nix`) merges `programs.transcriber.config` *onto* the existing `config.json` with jq at activation, rather than linking a read-only file, so GUI-edited vocabulary survives a switch. Declaring a field there that the GUI also owns will clobber the GUI's value on every switch.

`config::save()` writes atomically (temp file + rename) with mode 0600; `config::update(f)` applies an edit to a fresh read, which is how the settings app avoids reverting changes made elsewhere.

## Extension Development

The GNOME extension is plain JavaScript/GJS in `extension/`. After editing, reinstall with `bash install.sh` and restart GNOME Shell (Alt+F2 → `r` on X11, or log out/in on Wayland).
