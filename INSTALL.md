# Installation

## Prerequisites

### System packages (Fedora)

```bash
sudo dnf install -y \
    alsa-lib-devel \
    gtk4-devel \
    libadwaita-devel \
    glib2-devel
```

### Rust toolchain

Rust 1.92 or later is required (`rust-toolchain.toml` pins it; rustup picks it up automatically).

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

1. Build `transcriber-daemon` and `transcriber-settings` in release mode
2. Install both binaries to `~/.local/bin/`
3. Install the systemd user service to `~/.config/systemd/user/`
4. Install the DBus activation file to `~/.local/share/dbus-1/services/`
5. Install the `.desktop` launcher to `~/.local/share/applications/`
6. Install the GNOME Shell extension to `~/.local/share/gnome-shell/extensions/transcriber@local/`
7. Compile GSettings schemas
8. Enable and start the daemon via systemd

---

## Post-installation setup

### 1. Configure your API key

Open the Settings app:

```bash
transcriber-settings
```

Or launch **Transcriber Settings** from the GNOME application launcher.

Enter your API key and adjust any other settings (model, language, sample rate). Settings are saved to `~/.config/transcriber/config.json`.

### 2. Enable the GNOME extension

```bash
gnome-extensions enable transcriber@local
```

Alternatively, open the **Extensions** app and toggle **Transcriber** on.

If the extension does not appear in the list, restart GNOME Shell first:
- Press **Alt+F2**, type `r`, press **Enter** (X11 only)
- Or log out and back in (Wayland)

### 3. Use it

Press **Super+'** (Super + apostrophe) to start recording. Press it again to stop and transcribe.

The default keybinding is `<Super>apostrophe`. Rebind it in the extension preferences (`gnome-extensions prefs transcriber@local`). Escape cancels, and is only grabbed while a recording is in progress. The preferences also offer push-to-talk and a re-paste-last shortcut.

### Other Wayland desktops (Hyprland, Sway, KDE)

On a non-GNOME session `install.sh` also builds and installs the overlay, but doesn't enable it:

```bash
systemctl --user enable --now transcriber-overlay.service
```

The portal will propose Super+' as the default shortcut.

---

## Verify the daemon is running

```bash
systemctl --user status transcriber-daemon.service
```

To follow live logs:

```bash
journalctl --user -u transcriber-daemon.service -f
```

---

## Uninstall

```bash
# Stop and disable the service
systemctl --user disable --now transcriber-daemon.service

# Remove installed files
rm -f ~/.local/bin/transcriber-daemon
rm -f ~/.local/bin/transcriber-settings
rm -f ~/.config/systemd/user/transcriber-daemon.service
rm -f ~/.local/share/dbus-1/services/org.transcriber.Daemon.service
rm -f ~/.local/share/applications/org.transcriber.Settings.desktop
rm -rf ~/.local/share/gnome-shell/extensions/transcriber@local

# Optionally remove config and history
rm -rf ~/.config/transcriber
rm -rf ~/.local/share/transcriber

# Reload systemd
systemctl --user daemon-reload
```

---

## Update

For a source install, run:

```bash
bash update.sh
```

It pulls, rebuilds, reinstalls binaries and unit files that changed, and restarts the daemon when needed. Source installs never auto-update: `install.sh` marks the version `+git` and leaves the release update timer off, so a release build can't silently replace your local one.

Release installs (`install-remote.sh`) update via a daily timer. Each download is checked against the release's `SHA256SUMS` before installing. The installer needs `jq`, and never stores your `gh` token; it only saves a token you paste (fine-grained, read-only is enough) to `~/.config/transcriber/github-token` with mode 0600.

---

## Development build

To build without installing:

```bash
cargo build --release
```

Run components individually:

```bash
# Daemon (foreground, with logging)
RUST_LOG=debug cargo run -p transcriber-daemon

# Settings app
cargo run -p transcriber-settings
```
