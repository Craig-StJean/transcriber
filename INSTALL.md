# Installation

## Prerequisites

### System packages (Fedora)

```bash
sudo dnf install -y \
    alsa-lib-devel \
    gtk4-devel \
    libadwaita-devel \
    blueprint-compiler \
    glib2-devel
```

### Rust toolchain

Rust 1.92 or later is required.

```bash
# Install rustup if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Or update an existing installation
rustup update stable
```

Verify the version:
```bash
rustc --version   # should be >= 1.92
```

### API key

You need an API key for a Whisper-compatible transcription service.

- **Groq** (recommended — fast and free tier available): <https://console.groq.com>
- **Cohere** (excellent WER and user preference): <https://dashboard.cohere.com>
- **OpenAI**: <https://platform.openai.com>
- Any self-hosted OpenAI-compatible Whisper server also works.

---

## Install

Clone or download the repository, then run the installer from the repo root:

```bash
bash install.sh
```

The script will:

1. Build `voice-transcriber-daemon` and `voice-transcriber-settings` in release mode
2. Install both binaries to `~/.local/bin/`
3. Install the systemd user service to `~/.config/systemd/user/`
4. Install the DBus activation file to `~/.local/share/dbus-1/services/`
5. Install the `.desktop` launcher to `~/.local/share/applications/`
6. Install the GNOME Shell extension to `~/.local/share/gnome-shell/extensions/voice-transcriber@local/`
7. Compile GSettings schemas
8. Enable and start the daemon via systemd

---

## Post-installation setup

### 1. Configure your API key

Open the Settings app:

```bash
voice-transcriber-settings
```

Or launch **Voice Transcriber Settings** from the GNOME application launcher.

Enter your API key and adjust any other settings (model, language, sample rate). Settings are saved to `~/.config/voice-transcriber/config.json`.

### 2. Enable the GNOME extension

```bash
gnome-extensions enable voice-transcriber@local
```

Alternatively, open the **Extensions** app and toggle **Voice Transcriber** on.

If the extension does not appear in the list, restart GNOME Shell first:
- Press **Alt+F2**, type `r`, press **Enter** (X11 only)
- Or log out and back in (Wayland)

### 3. Use it

Press **Super+'** (Super + apostrophe) to start recording. Press it again to stop and transcribe.

The default keybinding is `<Super>apostrophe`. Rebind it via the **Settings** app, the extension preferences, or GNOME Settings → Keyboard.

---

## Verify the daemon is running

```bash
systemctl --user status voice-transcriber-daemon.service
```

To follow live logs:

```bash
journalctl --user -u voice-transcriber-daemon.service -f
```

---

## Uninstall

```bash
# Stop and disable the service
systemctl --user disable --now voice-transcriber-daemon.service

# Remove installed files
rm -f ~/.local/bin/voice-transcriber-daemon
rm -f ~/.local/bin/voice-transcriber-settings
rm -f ~/.config/systemd/user/voice-transcriber-daemon.service
rm -f ~/.local/share/dbus-1/services/org.transcriber.Daemon.service
rm -f ~/.local/share/applications/org.transcriber.Settings.desktop
rm -rf ~/.local/share/gnome-shell/extensions/voice-transcriber@local

# Optionally remove config and history
rm -rf ~/.config/voice-transcriber
rm -rf ~/.local/share/voice-transcriber

# Reload systemd
systemctl --user daemon-reload
```

---

## Update

After pulling new code, re-run the installer:

```bash
bash install.sh
```

Then restart the daemon to pick up the new binary:

```bash
systemctl --user restart voice-transcriber-daemon.service
```

---

## Development build

To build without installing:

```bash
cargo build --release
```

Run components individually:

```bash
# Daemon (foreground, with logging)
RUST_LOG=debug cargo run -p voice-transcriber-daemon

# Settings app
cargo run -p voice-transcriber-settings
```
