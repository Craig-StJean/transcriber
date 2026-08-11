# 🎙️ Voice Transcription Tool (Fedora / GNOME / Rust) — Architecture & Tech Plan

## 🎯 Primary Goal
Build a native-feeling Linux voice transcription tool designed ground-up for Wayland and GNOME. It uses a headless Rust daemon for the heavy lifting (audio capture, networking, hotkeys), XDG Desktop Portals for secure system-wide hotkeys, and a native GNOME Shell Extension for a flawless, always-on-top overlay. The architecture must remain decoupled so that alternative Wayland compositors can be supported in the future without rewriting the core logic.

## ⚙️ Prerequisites
* **Rust ≥ 1.92** — required by gtk4 0.11 / libadwaita 0.9. Run `rustup update stable` if on an older version.
* **System dev libraries:** `sudo dnf install -y alsa-lib-devel gtk4-devel libadwaita-devel blueprint-compiler`

---

# 🧠 Architecture Overview (The "Decoupled" Approach)

To survive Wayland's strict security model and remain portable, the "Brain" (audio, network, hotkeys) is entirely separated from the "Eyes" (the UI). **DBus is the glue that connects them.**

### 1. The Headless Daemon (`daemon/` - Rust)
* **Role:** The core engine. It runs in the background with no graphical UI.
* **Audio:** Captures mic input directly into an in-memory buffer using the cross-platform `cpal` library.
* **Hotkeys:** Global shortcuts are registered by the GNOME extension via `Main.wm.addKeybinding()`, backed by a GSettings schema. This stores keybindings in dconf (editable in GNOME Settings or the extension prefs panel). The XDG `GlobalShortcuts` portal (`ashpd`) is reserved for a future Phase 2 supporting non-GNOME Wayland compositors (Sway, Hyprland).
* **IPC (Inter-Process Communication):** Exposes a `zbus` DBus interface (`org.transcriber.Daemon`) to listen for commands and broadcast state changes (e.g., `StateChanged(Recording)`, `AudioLevel(f32)`).
* **Networking:** Sends the buffered audio to an OpenAI-compatible API (e.g., Groq) via `reqwest`.
* **Clipboard:** Injects the resulting text into the Wayland clipboard via `wl-clipboard-rs`.

### 2. The GNOME Overlay (`extension/` - JavaScript/GJS)
* **Role:** The frontend UI tailored specifically for GNOME users.
* **Tech:** GNOME Shell Extension using Clutter/St (GNOME's native UI toolkit).
* **Behavior:** Purely a dumb frontend. It listens to the Rust daemon's DBus signals. When it hears `StateChanged(Recording)`, it draws a borderless "Listening..." overlay that sits perfectly above all windows. It intercepts the `Esc` key natively to send a `Cancel()` DBus call back to the daemon.

### 3. Settings & History App (`app/` - Rust)
* **Role:** User configuration and viewing past transcriptions.
* **Tech:** GTK4 + Libadwaita + Blueprint.
* **Behavior:** Reads/writes to `~/.config/transcriber/config.json`. Talks to the daemon via DBus to trigger retries or update settings on the fly. 

---

# 🛑 Strict Warnings: What NOT To Do Under Wayland

To save you days of frustration on Fedora/GNOME, avoid these common Linux desktop pitfalls:

* ❌ **DO NOT use `gtk4-layer-shell` for the GNOME UI.**
  * *Why:* GNOME (Mutter) explicitly rejects the `wlr-layer-shell` protocol. Your overlay will either crash or spawn as a regular, decorated window. You *must* use a GNOME Extension for a true overlay on Fedora. 
* ❌ **DO NOT write a custom global keylogger (e.g., reading `/dev/input`).**
  * *Why:* Wayland blocks background apps from reading keystrokes. Reading `/dev/input` requires root privileges or messy `udev` group rules. On GNOME, use `Main.wm.addKeybinding()` in the extension, backed by a GSettings schema. On non-GNOME Wayland, use the XDG `GlobalShortcuts` portal (`ashpd`).
* ❌ **DO NOT use `pipewire-rs` directly unless absolutely necessary.**
  * *Why:* Native PipeWire bindings are extremely low-level and require manual buffer management and C-style unsafe code. `cpal` wraps this perfectly for safe, reliable audio capture.
* ❌ **DO NOT write temporary `.wav` files to disk.**
  * *Why:* Disk I/O introduces latency and requires cleanup logic. Buffer your 16-bit PCM audio directly in memory (e.g., using `std::io::Cursor` in Rust) and send the bytes directly to the API.
* ❌ **DO NOT rely on `arboard` for Wayland background clipboard injection.**
  * *Why:* Wayland often denies clipboard access to apps that don't have an active, focused window. Use `wl-clipboard-rs` which interacts directly with the Wayland compositor to force clipboard updates.
* ❌ **DO NOT run the daemon as a plain process without importing Wayland env vars into systemd.**
  * *Why:* `wl-clipboard-rs` forks a child process that requires `WAYLAND_DISPLAY` and `XDG_RUNTIME_DIR`. Systemd user services do not inherit these by default — the service must call `systemctl --user import-environment` before starting, otherwise clipboard writes silently fail.
* ❌ **DO NOT use the `hound` crate for WAV encoding.**
  * *Why:* `hound` is unmaintained. A standard RIFF/WAV header is only 44 bytes and trivially written with `std::io::Write`. No extra dependency needed.

---

# 🧩 Technology Stack Justification

## Backend / Daemon (Rust)
* **`cpal`**: The best cross-platform audio library for Rust. Abstracts ALSA/PulseAudio/PipeWire into a clean stream of `f32` samples (captured as `f32`, converted to `i16` for WAV encoding).
* **`zbus`**: The standard for DBus in Rust. Exposes the daemon's API that the GNOME extension and Settings app consume.
* **`reqwest`** (with `rustls` feature, no OpenSSL): Fast, async HTTP client for the transcription API. `serde_json` parses the response.
* **`wl-clipboard-rs`**: Bypasses GTK clipboard limitations by talking directly to the Wayland compositor clipboard protocol.
* **`rusqlite`** (with `bundled` feature): Lightweight embedded SQLite for transcription history. No system `libsqlite3` required.
* **`tokio`**: Async runtime for concurrent network requests, DBus signals, and audio streaming.
* **`ashpd`** *(Phase 2 only)*: XDG Desktop Portal wrapper. Reserved for the future non-GNOME overlay (`overlay/` binary) targeting wlroots compositors where `Main.wm` is unavailable.

## Settings App (Rust)
* **`gtk4` & `libadwaita`**: The standard for modern, native-looking GNOME applications. 
* **`blueprint-compiler`**: A clean, declarative markup language for designing GTK4 interfaces (much easier to read and write than XML).

## Overlay (GNOME)
* **JavaScript (ES6) / GJS**: The required language for GNOME Shell Extensions.
* **Clutter / St**: GNOME's internal UI toolkits. Allows drawing raw UI elements directly onto the compositor screen, bypassing all Wayland window restrictions.

---

# 🔄 Core Data Workflow

1. **User presses the global hotkey** (configured previously via the native XDG Portal prompt).
2. The Wayland Compositor catches the key and triggers the XDG portal, which signals the **Rust Daemon**.
3. **Daemon:**
   * Transitions internal state to `Recording`.
   * Emits DBus signal: `StateChanged("Recording")`.
   * Starts `cpal` audio stream, capturing chunks into a memory buffer.
4. **GNOME Extension (UI):**
   * Hears the DBus signal.
   * Renders the "Listening..." overlay on screen.
5. **User presses hotkey again.**
6. **Daemon:**
   * Stops `cpal` stream.
   * Emits DBus signal: `StateChanged("Transcribing")`.
   * Wraps the memory buffer in WAV headers and HTTP POSTs it to the API.
7. **API responds with text.**
8. **Daemon:**
   * Writes text to the clipboard via `wl-clipboard-rs`.
   * Saves the text and timestamp to the SQLite history database.
   * Emits DBus signal: `StateChanged("Done")`.
9. **GNOME Extension (UI):**
   * Updates overlay to "Copied to clipboard!", then fades out.

---

# 🔮 Future Upgrades & Expansions

Because the architecture is modular and heavily relies on DBus, adding future capabilities becomes much easier:

## 1. Other Desktop Environment Support (wlroots / KDE)

**Status: Implemented** — `overlay/` binary added to the workspace.

The `overlay/` Rust binary provides a native Wayland overlay for any compositor that supports the `wlr-layer-shell-unstable-v1` protocol: Sway, Hyprland, River, KDE Plasma 6, and others. It listens to the **exact same DBus signals** as the GNOME extension — the daemon is untouched.

### System packages required (build + runtime)
```bash
sudo dnf install -y gtk4-layer-shell-devel   # build
sudo dnf install -y gtk4-layer-shell         # runtime .so
```

### New crate: `overlay/`

| File | Role |
|---|---|
| `overlay/src/main.rs` | GTK4 app entry point; bridges Tokio ↔ GTK via `glib::MainContext::channel` |
| `overlay/src/daemon_proxy.rs` | `#[zbus::proxy]` for `org.transcriber.Daemon`; streams `StateChanged`, `AudioLevel`, `TranscriptionReady` to the GTK thread |
| `overlay/src/shortcuts.rs` | XDG `GlobalShortcuts` portal via `ashpd`; maps activations to `StartRecording` / `StopRecording` / `Cancel` DBus calls |
| `overlay/src/overlay_window.rs` | `gtk4-layer-shell` window anchored to the bottom-centre of the screen; VU bars + status label; generation-based fade animation |
| `deploy/transcriber-overlay.service` | Systemd user service (mirrors the daemon service) |

### Key dependencies
* **`gtk4-layer-shell = "0.8"`** — Rust bindings for the C `gtk4-layer-shell` library. Implements the `wlr-layer-shell-unstable-v1` Wayland protocol. Provides the `LayerShell` trait on `gtk::ApplicationWindow`.
* **`ashpd = { version = "0.11", features = ["tokio"] }`** — XDG Desktop Portal wrapper. `GlobalShortcuts` portal replaces `Main.wm.addKeybinding()` for non-GNOME Wayland.
* **`gtk4 = "0.9"`** — GTK4 toolkit (intentionally separate from the settings app's `"0.11"` since the overlay doesn't use Blueprint or libadwaita).
* **`futures-util = "0.3"`** — `StreamExt` for consuming zbus signal streams.

### Architecture

```
┌─────────────────────────────────────────────┐
│  GTK main thread                            │
│  OverlayWindow (layer-shell ApplicationWindow) │
│  ← glib::MainContext::channel ←             │
│       ┌──────────────────────────────────┐  │
│       │  Tokio thread                    │  │
│       │  daemon_proxy::connect()         │  │
│       │    ├── StateChanged stream       │  │
│       │    ├── AudioLevel stream         │  │
│       │    └── TranscriptionReady stream │  │
│       │  shortcuts::run()                │  │
│       │    └── ashpd Activated loop ──→  │  │
│       │        proxy.start/stop/cancel() │  │
│       └──────────────────────────────────┘  │
└─────────────────────────────────────────────┘
                      ↕ DBus
                  daemon binary
```

### Clipboard injection
On GNOME the extension uses `St.Clipboard.get_default().set_text()`.  On
wlroots the overlay calls `gdk::Display::default().clipboard().set_text()` when
it receives the `TranscriptionReady` signal — this works because the overlay
holds an active Wayland display connection via GTK4.  No extra `wl-clipboard-rs`
dependency is needed.

### Hotkey registration
The XDG `GlobalShortcuts` portal shows a **one-time approval dialog** on first
launch.  After the user approves, the shortcuts are stored by the portal and
activated across reboots without further prompts.  If the portal is unavailable
(compositor doesn't support it), the binary logs a warning and continues without
hotkeys — the overlay will still respond to DBus signals normally.

### Build & install
```bash
# Install build dependency
sudo dnf install -y gtk4-layer-shell-devel

# Build
cargo build --release -p transcriber-overlay

# Install binary
install -Dm755 target/release/transcriber-overlay ~/.local/bin/

# Enable systemd service (on a wlroots/KDE session)
systemctl --user enable --now transcriber-overlay
```

### Compatibility notes
* **GNOME / Mutter** — Mutter rejects `wlr-layer-shell`.  The binary detects
  this at startup via `gtk4_layer_shell::is_layer_shell_supported()` and logs an
  error. Continue using the GNOME Shell Extension on GNOME.
* **KDE Plasma 5.27+** — Partial `wlr-layer-shell` support via a compatibility
  shim; layer positioning works but keyboard interaction may be limited.
* **KDE Plasma 6** — Full `wlr-layer-shell` and `GlobalShortcuts` portal support.

## 3. Smart Silence Detection (VAD)
Integrate Voice Activity Detection (e.g., `silero-vad`). Instead of requiring the user to press the hotkey a second time to stop recording, the daemon analyzes the `cpal` audio stream in real-time and automatically stops and transcribes when it detects 1-2 seconds of silence. There would be a setting to enable this in the GTK app.

## 4. Direct Text Injection (Typing Simulation)
Instead of copying the transcription to the clipboard and making the user paste it (`Ctrl+V`), use Wayland's `virtual-keyboard-v1` protocol (on wlroots) or the `org.gnome.Mutter.RemoteDesktop` DBus API (on GNOME) to simulate keystrokes. This allows the tool to type the transcribed text directly into the currently focused text box.

## 5. Audio Feedback Cues
Play subtle, satisfying UI sounds (e.g., a soft "ding") when recording starts and successfully finishes, providing feedback without needing to look at the visual overlay.

## 6. Real-time Streaming Transcription
Instead of waiting for the recording to finish, stream the audio chunks to the API via WebSockets or gRPC, allowing the GNOME overlay to display the transcribed text word-by-word as the user speaks.

## 7. GNOME 50 / libadwaita 1.9 Adoption — Done

**Status: Implemented** (May 2026, GNOME 50 / libadwaita 1.9.0 / gtk4 4.22).

Adopted:
- `app/Cargo.toml` — feature flag bumped from `v1_8` → `v1_9`.
- `extension/metadata.json` — `shell-version` extended to include `"50"`; manifest version bumped 1 → 2.
- `app/src/main.rs` — added `AdwShortcutsDialog` (a libadwaita 1.8 widget) accessible from the menu and via `Ctrl+?`. Reads global hotkeys live from the extension's GSettings schema so user rebinds reflect immediately, with sensible fallbacks if the extension isn't installed.
- `extension/overlay.js` — gates the fade-in / fade-out `ease()` calls on `St.Settings.get().enable_animations`, honouring GNOME's "Reduce Animation" accessibility toggle.

Speculative items from earlier drafts that were **NOT** adopted, with reasons:
- **`AdwSidebar` for the History page** — the as-shipped 1.9 widget is a navigation widget designed for `AdwNavigationSplitView` / `AdwOverlaySplitView` (see `docs/libadwaita/04-navigation.md:197`). The history page is a list of past transcriptions with per-row Copy/Retry/Play/Delete buttons; `GtkListBox` with `.boxed-list` is still the documented pattern for that role (`docs/libadwaita/15-widget-selection-and-migration.md:217`).
- **`AdwViewSwitcherSidebar`** — recommended for 6+ views or sectioned navigation; this app has 3 tabs, where `AdwInlineViewSwitcher` is the right HIG pattern.